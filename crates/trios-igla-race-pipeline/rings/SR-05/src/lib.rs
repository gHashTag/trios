//! SR-05 — Railway Deployer (fleet integration contract + R7 audit triplet).
//!
//! GOLD I / SR-05. Defines the `RailwayApi` trait, the
//! `RailwayDeployer` state machine (observe-only by default), the
//! `FleetSnapshot` / `DeployReceipt` / `AuditReport` wire types, and
//! the R7 audit triplet formatter
//! (`RAIL=<verb> @ project=<8c> service=<8c> sha=<8c> ts=<rfc3339>`).
//!
//! ## Honest scope (R5)
//!
//! This ring ships the **contract + state machine + R7 formatter**.
//! The concrete `reqwest` GraphQL client (Railway API) and optional
//! `sqlx` audit-log writer ship in a sibling BR-IO ring. Pulling
//! reqwest with rustls + a git dep on `trios-railway-audit` into a
//! Silver ring would violate `R-RING-DEP-002` and break offline cargo
//! builds. Same precedent as `trios_kg::KgClient` for SR-MEM-01.
//!
//! Issue AC's smoke-test against a Railway dev project requires
//! `RAILWAY_TOKEN_TEST`; that runs in the BR-IO adapter PR.
//!
//! Closes #458 · Part of #446 · Anchor: phi^2 + phi^-2 = 3

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, instrument, warn};

// ─────────────── modes ───────────────

/// Deployer mode. Defaults to `Observe`. The issue AC gates `Apply`
/// behind `DEPLOYER_MODE=apply`; that env-var check ships at the BR-IO
/// adapter boundary that performs the real GraphQL mutation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployerMode {
    /// Read-only mode. No mutations are issued.
    #[default]
    Observe,
    /// Mutating mode. The BR-IO adapter MUST cross-check
    /// `DEPLOYER_MODE=apply` before committing a write.
    Apply,
}

// ─────────────── fleet types ───────────────

/// One Railway service in a project. Mirrors the `trios-railway-audit`
/// shape without depending on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RailwayService {
    /// Railway project id (UUID-shaped string).
    pub project_id: String,
    /// Railway service id (UUID-shaped string).
    pub service_id: String,
    /// Optional service name (free-form).
    pub name: Option<String>,
    /// Currently-deployed git SHA, if any.
    pub sha: Option<String>,
    /// Status string from Railway (e.g. `RUNNING`, `STOPPED`).
    pub status: String,
}

/// A snapshot of services in one Railway project.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FleetSnapshot {
    /// Project id this snapshot is rooted at.
    pub project_id: String,
    /// Services live in the project at snapshot time.
    pub services: Vec<RailwayService>,
}

// ─────────────── deploy receipt ───────────────

/// One concrete action the deployer would take (or did take, in Apply
/// mode).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verb", rename_all = "snake_case")]
pub enum DeployAction {
    /// Service is missing in live → create.
    Create {
        /// Project id.
        project_id: String,
        /// Service id (placeholder if not yet allocated).
        service_id: String,
        /// Target SHA.
        sha: String,
    },
    /// Service exists but SHA differs → redeploy.
    Redeploy {
        /// Project id.
        project_id: String,
        /// Service id.
        service_id: String,
        /// New target SHA.
        sha: String,
    },
    /// Service is in live but not in desired → tear down.
    Teardown {
        /// Project id.
        project_id: String,
        /// Service id.
        service_id: String,
    },
    /// Service matches — no-op.
    Noop {
        /// Project id.
        project_id: String,
        /// Service id.
        service_id: String,
    },
}

impl DeployAction {
    /// `RAIL=<verb>` field of the audit triplet.
    pub fn verb(&self) -> &'static str {
        match self {
            Self::Create { .. } => "create",
            Self::Redeploy { .. } => "redeploy",
            Self::Teardown { .. } => "teardown",
            Self::Noop { .. } => "noop",
        }
    }
}

/// R7 audit triplet (per `trios-railway/AGENTS.md`):
/// `RAIL=<verb> @ project=<8c> service=<8c> sha=<8c> ts=<rfc3339>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R7Triplet {
    /// Verb: `create` / `redeploy` / `teardown` / `noop` / `audit`.
    pub verb: String,
    /// First 8 chars of project_id.
    pub project_8c: String,
    /// First 8 chars of service_id.
    pub service_8c: String,
    /// First 8 chars of sha (or empty for Teardown / Noop).
    pub sha_8c: String,
    /// RFC-3339 timestamp.
    pub ts: DateTime<Utc>,
}

impl R7Triplet {
    /// Build a triplet from a [`DeployAction`]. Trims project / service
    /// / sha to 8 chars; refuses any field shorter than 8 with
    /// [`DeployErr::ShaTooShort`].
    pub fn from_action(action: &DeployAction, ts: DateTime<Utc>) -> Result<Self, DeployErr> {
        let (project_id, service_id, sha) = match action {
            DeployAction::Create { project_id, service_id, sha } => (project_id, service_id, sha.as_str()),
            DeployAction::Redeploy { project_id, service_id, sha } => (project_id, service_id, sha.as_str()),
            DeployAction::Teardown { project_id, service_id } => (project_id, service_id, ""),
            DeployAction::Noop { project_id, service_id } => (project_id, service_id, ""),
        };
        if project_id.len() < 8 || service_id.len() < 8 {
            return Err(DeployErr::ShaTooShort);
        }
        if !sha.is_empty() && sha.len() < 8 {
            return Err(DeployErr::ShaTooShort);
        }
        Ok(Self {
            verb: action.verb().to_string(),
            project_8c: project_id[..8].to_string(),
            service_8c: service_id[..8].to_string(),
            sha_8c: if sha.is_empty() { String::new() } else { sha[..8].to_string() },
            ts,
        })
    }

    /// Render in the canonical wire form
    /// `RAIL=<verb> @ project=<8c> service=<8c> sha=<8c> ts=<rfc3339>`.
    pub fn format(&self) -> String {
        format!(
            "RAIL={} @ project={} service={} sha={} ts={}",
            self.verb,
            self.project_8c,
            self.service_8c,
            self.sha_8c,
            self.ts.to_rfc3339()
        )
    }
}

/// Receipt returned by [`RailwayDeployer::deploy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployReceipt {
    /// Mode the deployer ran in.
    pub mode: DeployerModeSerial,
    /// One [`DeployAction`] per service comparison.
    pub actions: Vec<DeployAction>,
    /// One [`R7Triplet`] per non-noop action (audit trail).
    pub triplets: Vec<R7Triplet>,
}

/// Serialisable mirror of [`DeployerMode`] (for the receipt).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployerModeSerial {
    /// Observe.
    #[default]
    Observe,
    /// Apply.
    Apply,
}

impl From<DeployerMode> for DeployerModeSerial {
    fn from(m: DeployerMode) -> Self {
        match m {
            DeployerMode::Observe => Self::Observe,
            DeployerMode::Apply => Self::Apply,
        }
    }
}

// ─────────────── audit report ───────────────

/// Drift report from [`RailwayDeployer::audit`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReport {
    /// Total number of services that drifted.
    pub drift_count: usize,
    /// Services in desired but missing from live (would Create).
    pub missing: Vec<String>,
    /// Services in live but absent from desired (would Teardown).
    pub extra: Vec<String>,
    /// Services present in both with mismatched SHA (would Redeploy).
    pub sha_mismatch: Vec<String>,
}

// ─────────────── errors ───────────────

/// Deploy-side errors.
#[derive(Debug, Error)]
pub enum DeployErr {
    /// Backend (Railway API) returned a hard error.
    #[error("backend error: {0}")]
    Backend(String),
    /// Caller asked for a mutation but the deployer is in Observe mode.
    #[error("mutation refused: deployer is in observe-only mode")]
    ModeRefused,
    /// project_id / service_id / sha shorter than 8 chars (R7 triplet
    /// requires 8 chars per field).
    #[error("project / service / sha must be at least 8 chars for R7 triplet")]
    ShaTooShort,
}

// ─────────────── RailwayApi trait ───────────────

/// Async Railway API surface the deployer issues calls against.
/// Implemented by `MockApi` (this file's tests) and by a concrete
/// `reqwest`-backed adapter in the sibling BR-IO ring.
pub trait RailwayApi: Send + Sync {
    /// Read-only: snapshot the live fleet for one project.
    fn list_services<'a>(
        &'a self,
        project_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<RailwayService>, String>> + Send + 'a>>;

    /// Mutating: create a new service at the given SHA.
    fn create_service<'a>(
        &'a self,
        project_id: &'a str,
        sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;

    /// Mutating: redeploy an existing service to a new SHA.
    fn redeploy_service<'a>(
        &'a self,
        project_id: &'a str,
        service_id: &'a str,
        sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

    /// Mutating: tear down a service.
    fn teardown_service<'a>(
        &'a self,
        project_id: &'a str,
        service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

// ─────────────── RailwayDeployer ───────────────

/// Deployer state machine. Observe-only by default.
pub struct RailwayDeployer<A: RailwayApi> {
    api: A,
    mode: DeployerMode,
}

impl<A: RailwayApi> RailwayDeployer<A> {
    /// Build a new deployer.
    pub fn new(api: A, mode: DeployerMode) -> Self {
        Self { api, mode }
    }

    /// Build an observe-only deployer (matches the issue AC default).
    pub fn observe(api: A) -> Self {
        Self::new(api, DeployerMode::Observe)
    }

    /// Active mode.
    pub fn mode(&self) -> DeployerMode {
        self.mode
    }

    /// Snapshot the live fleet for one project.
    #[instrument(skip(self))]
    pub async fn fleet_snapshot(&self) -> Result<FleetSnapshot, DeployErr> {
        // Caller hasn't supplied a project_id; the issue AC is for one
        // project at a time, so callers should use
        // `fleet_snapshot_for(project_id)` below. We keep this method
        // returning an empty snapshot for callers that want to start
        // fresh.
        Ok(FleetSnapshot::default())
    }

    /// Snapshot the live fleet for the supplied project.
    #[instrument(skip(self))]
    pub async fn fleet_snapshot_for(&self, project_id: &str) -> Result<FleetSnapshot, DeployErr> {
        let services = self
            .api
            .list_services(project_id)
            .await
            .map_err(DeployErr::Backend)?;
        Ok(FleetSnapshot {
            project_id: project_id.to_string(),
            services,
        })
    }

    /// Read-only audit pass. Reports drift between desired and live.
    #[instrument(skip(self, desired))]
    pub async fn audit(&self, desired: &FleetSnapshot) -> Result<AuditReport, DeployErr> {
        let live = self.fleet_snapshot_for(&desired.project_id).await?;
        let mut report = AuditReport::default();
        let live_ids: Vec<&String> = live.services.iter().map(|s| &s.service_id).collect();
        let desired_ids: Vec<&String> = desired.services.iter().map(|s| &s.service_id).collect();

        for d in desired.services.iter() {
            match live.services.iter().find(|s| s.service_id == d.service_id) {
                None => {
                    report.missing.push(d.service_id.clone());
                    report.drift_count += 1;
                }
                Some(s) if s.sha != d.sha => {
                    report.sha_mismatch.push(d.service_id.clone());
                    report.drift_count += 1;
                }
                Some(_) => {}
            }
        }
        for s in live.services.iter() {
            if !desired_ids.iter().any(|id| **id == s.service_id) {
                report.extra.push(s.service_id.clone());
                report.drift_count += 1;
            }
        }
        let _ = live_ids;
        Ok(report)
    }

    /// Compute (and optionally execute) the deploy plan for `desired`.
    /// In `Observe` mode every action is logged but no mutation is
    /// issued. In `Apply` mode mutations are dispatched through the
    /// RailwayApi trait.
    #[instrument(skip(self, desired))]
    pub async fn deploy(&self, desired: &FleetSnapshot) -> Result<DeployReceipt, DeployErr> {
        let live = self.fleet_snapshot_for(&desired.project_id).await?;
        let mut actions: Vec<DeployAction> = Vec::new();

        // 1. desired services: create if missing, redeploy if drift, noop if match.
        for d in desired.services.iter() {
            let sha = d.sha.clone().unwrap_or_default();
            match live.services.iter().find(|s| s.service_id == d.service_id) {
                None => actions.push(DeployAction::Create {
                    project_id: desired.project_id.clone(),
                    service_id: d.service_id.clone(),
                    sha,
                }),
                Some(s) if s.sha != d.sha => actions.push(DeployAction::Redeploy {
                    project_id: desired.project_id.clone(),
                    service_id: d.service_id.clone(),
                    sha,
                }),
                Some(_) => actions.push(DeployAction::Noop {
                    project_id: desired.project_id.clone(),
                    service_id: d.service_id.clone(),
                }),
            }
        }
        // 2. live services missing from desired → teardown.
        for s in live.services.iter() {
            if !desired.services.iter().any(|d| d.service_id == s.service_id) {
                actions.push(DeployAction::Teardown {
                    project_id: desired.project_id.clone(),
                    service_id: s.service_id.clone(),
                });
            }
        }

        // 3. Execute (Apply) or skip (Observe).
        let now = Utc::now();
        let mut triplets: Vec<R7Triplet> = Vec::new();
        for action in actions.iter() {
            // Build R7 triplet for every non-noop action.
            if !matches!(action, DeployAction::Noop { .. }) {
                triplets.push(R7Triplet::from_action(action, now)?);
            }
            if matches!(self.mode, DeployerMode::Apply) {
                match action {
                    DeployAction::Create { project_id, sha, .. } => {
                        self.api
                            .create_service(project_id, sha)
                            .await
                            .map_err(DeployErr::Backend)?;
                    }
                    DeployAction::Redeploy { project_id, service_id, sha } => {
                        self.api
                            .redeploy_service(project_id, service_id, sha)
                            .await
                            .map_err(DeployErr::Backend)?;
                    }
                    DeployAction::Teardown { project_id, service_id } => {
                        self.api
                            .teardown_service(project_id, service_id)
                            .await
                            .map_err(DeployErr::Backend)?;
                    }
                    DeployAction::Noop { .. } => {
                        debug!("noop");
                    }
                }
            } else {
                warn!(verb = action.verb(), "observe mode — skipping mutation");
            }
        }

        Ok(DeployReceipt {
            mode: self.mode.into(),
            actions,
            triplets,
        })
    }
}

// ─────────────── tests ───────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// In-memory MockApi with controllable failure injection + mutation
    /// counters.
    struct MockApi {
        live: StdMutex<Vec<RailwayService>>,
        fail_list: StdMutex<bool>,
        creates: StdMutex<u32>,
        redeploys: StdMutex<u32>,
        teardowns: StdMutex<u32>,
    }

    impl MockApi {
        fn new(initial: Vec<RailwayService>) -> Self {
            Self {
                live: StdMutex::new(initial),
                fail_list: StdMutex::new(false),
                creates: StdMutex::new(0),
                redeploys: StdMutex::new(0),
                teardowns: StdMutex::new(0),
            }
        }
        fn break_list(&self) {
            *self.fail_list.lock().unwrap() = true;
        }
    }

    impl RailwayApi for MockApi {
        fn list_services<'a>(
            &'a self,
            _project_id: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<RailwayService>, String>> + Send + 'a>>
        {
            Box::pin(async move {
                if *self.fail_list.lock().unwrap() {
                    return Err("mock list failure".into());
                }
                Ok(self.live.lock().unwrap().clone())
            })
        }
        fn create_service<'a>(
            &'a self,
            _project_id: &'a str,
            _sha: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>
        {
            Box::pin(async move {
                *self.creates.lock().unwrap() += 1;
                Ok("svc-NEW00001".into())
            })
        }
        fn redeploy_service<'a>(
            &'a self,
            _project_id: &'a str,
            _service_id: &'a str,
            _sha: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
        {
            Box::pin(async move {
                *self.redeploys.lock().unwrap() += 1;
                Ok(())
            })
        }
        fn teardown_service<'a>(
            &'a self,
            _project_id: &'a str,
            _service_id: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
        {
            Box::pin(async move {
                *self.teardowns.lock().unwrap() += 1;
                Ok(())
            })
        }
    }

    fn svc(id: &str, sha: Option<&str>) -> RailwayService {
        RailwayService {
            project_id: "p1234567".into(),
            service_id: id.into(),
            name: None,
            sha: sha.map(String::from),
            status: "RUNNING".into(),
        }
    }

    fn fleet(services: Vec<RailwayService>) -> FleetSnapshot {
        FleetSnapshot {
            project_id: "p1234567".into(),
            services,
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 2, 13, 0, 0).single().unwrap_or_else(Utc::now)
    }

    use chrono::TimeZone;

    // ── R7 triplet tests ──

    #[test]
    fn r7_triplet_format_canonical() {
        let action = DeployAction::Create {
            project_id: "p1234567abcdef".into(),
            service_id: "s9876543ZYXW".into(),
            sha: "deadbeef00".into(),
        };
        let t = R7Triplet::from_action(&action, now()).unwrap();
        let s = t.format();
        assert!(s.starts_with("RAIL=create @ project=p1234567 service=s9876543 sha=deadbeef ts="));
        assert!(s.contains("ts=2026-05-02T13:00:00+00:00"));
    }

    #[test]
    fn r7_triplet_teardown_has_empty_sha() {
        let action = DeployAction::Teardown {
            project_id: "p1234567".into(),
            service_id: "s9876543".into(),
        };
        let t = R7Triplet::from_action(&action, now()).unwrap();
        assert_eq!(t.sha_8c, "");
        assert_eq!(t.verb, "teardown");
    }

    #[test]
    fn r7_triplet_rejects_short_project() {
        let action = DeployAction::Create {
            project_id: "short".into(),
            service_id: "s12345678".into(),
            sha: "abcdef01".into(),
        };
        match R7Triplet::from_action(&action, now()) {
            Err(DeployErr::ShaTooShort) => {}
            other => panic!("expected ShaTooShort, got {other:?}"),
        }
    }

    // ── DeployerMode tests ──

    #[test]
    fn default_mode_is_observe() {
        let mode = DeployerMode::default();
        assert_eq!(mode, DeployerMode::Observe);
    }

    #[tokio::test]
    async fn observe_mode_does_not_mutate() {
        let api = MockApi::new(vec![]);
        let deployer = RailwayDeployer::observe(api);
        let desired = fleet(vec![svc("s12345678", Some("abcdef01ff"))]);
        let receipt = deployer.deploy(&desired).await.unwrap();
        // One Create action planned, one R7 triplet emitted, but no
        // RailwayApi mutations issued.
        assert_eq!(receipt.actions.len(), 1);
        assert!(matches!(receipt.actions[0], DeployAction::Create { .. }));
        assert_eq!(receipt.triplets.len(), 1);
        assert_eq!(*deployer.api.creates.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn apply_mode_executes_mutations() {
        let api = MockApi::new(vec![svc("s_old0001", Some("oldoldoldold"))]);
        let deployer = RailwayDeployer::new(api, DeployerMode::Apply);
        // Desired wants one new + one redeploy; live's s_old0001 is
        // missing from desired → teardown.
        let desired = fleet(vec![svc("s_new0001", Some("newnewnewnew"))]);
        let receipt = deployer.deploy(&desired).await.unwrap();
        assert_eq!(receipt.actions.len(), 2); // create + teardown
        assert_eq!(*deployer.api.creates.lock().unwrap(), 1);
        assert_eq!(*deployer.api.teardowns.lock().unwrap(), 1);
    }

    // ── deploy plan tests ──

    #[tokio::test]
    async fn deploy_creates_missing_service() {
        let api = MockApi::new(vec![]);
        let deployer = RailwayDeployer::observe(api);
        let desired = fleet(vec![svc("s12345678", Some("abcdef01ff"))]);
        let receipt = deployer.deploy(&desired).await.unwrap();
        assert!(matches!(receipt.actions[0], DeployAction::Create { .. }));
    }

    #[tokio::test]
    async fn deploy_redeploys_on_sha_drift() {
        let api = MockApi::new(vec![svc("s12345678", Some("oldoldoldold"))]);
        let deployer = RailwayDeployer::observe(api);
        let desired = fleet(vec![svc("s12345678", Some("newnewnewnew"))]);
        let receipt = deployer.deploy(&desired).await.unwrap();
        assert!(matches!(receipt.actions[0], DeployAction::Redeploy { .. }));
    }

    #[tokio::test]
    async fn deploy_emits_noop_when_aligned() {
        let api = MockApi::new(vec![svc("s12345678", Some("samesamesame"))]);
        let deployer = RailwayDeployer::observe(api);
        let desired = fleet(vec![svc("s12345678", Some("samesamesame"))]);
        let receipt = deployer.deploy(&desired).await.unwrap();
        assert!(matches!(receipt.actions[0], DeployAction::Noop { .. }));
        // Noop does not emit an R7 triplet (audit trail captures
        // changes only).
        assert!(receipt.triplets.is_empty());
    }

    #[tokio::test]
    async fn deploy_tears_down_extra_live_service() {
        let api = MockApi::new(vec![
            svc("s12345678", Some("samesamesame")),
            svc("s_extra01", Some("extraextra11")),
        ]);
        let deployer = RailwayDeployer::observe(api);
        let desired = fleet(vec![svc("s12345678", Some("samesamesame"))]);
        let receipt = deployer.deploy(&desired).await.unwrap();
        // 1 noop + 1 teardown
        assert_eq!(receipt.actions.len(), 2);
        assert!(receipt
            .actions
            .iter()
            .any(|a| matches!(a, DeployAction::Teardown { .. })));
    }

    // ── audit tests ──

    #[tokio::test]
    async fn audit_reports_missing_extra_drift() {
        let api = MockApi::new(vec![
            svc("s12345678", Some("oldoldoldold")),
            svc("s_extra01", Some("extraextra11")),
        ]);
        let deployer = RailwayDeployer::observe(api);
        let desired = fleet(vec![
            svc("s12345678", Some("newnewnewnew")), // sha drift
            svc("s_missing", Some("missmissmiss")), // missing
        ]);
        let report = deployer.audit(&desired).await.unwrap();
        assert_eq!(report.drift_count, 3);
        assert_eq!(report.missing, vec!["s_missing".to_string()]);
        assert_eq!(report.sha_mismatch, vec!["s12345678".to_string()]);
        assert_eq!(report.extra, vec!["s_extra01".to_string()]);
    }

    // ── fleet_snapshot tests ──

    #[tokio::test]
    async fn fleet_snapshot_for_passes_through_api() {
        let api = MockApi::new(vec![svc("s12345678", Some("abcdef0123"))]);
        let deployer = RailwayDeployer::observe(api);
        let snap = deployer.fleet_snapshot_for("p1234567").await.unwrap();
        assert_eq!(snap.services.len(), 1);
        assert_eq!(snap.project_id, "p1234567");
    }

    #[tokio::test]
    async fn fleet_snapshot_propagates_backend_errors() {
        let api = MockApi::new(vec![]);
        api.break_list();
        let deployer = RailwayDeployer::observe(api);
        match deployer.fleet_snapshot_for("p1234567").await {
            Err(DeployErr::Backend(msg)) => assert!(msg.contains("mock list failure")),
            other => panic!("expected Backend, got {other:?}"),
        }
    }

    // ── φ-anchor ──

    #[test]
    fn phi_anchor_present() {
        let phi: f64 = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let lhs = phi * phi + 1.0 / (phi * phi);
        assert!((lhs - 3.0).abs() < 1e-10, "phi anchor violated: {lhs}");
    }
}
