use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentInfo {
    pub did: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Online,
    Offline,
    Busy,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BrowserCommandReq {
    Navigate { url: String },
    Click { selector: String },
    FillForm { selector: String, value: String },
    Screenshot,
    GetDom { selector: Option<String> },
    EvalJs { code: String },
}
