//! BRONZE-RING-DR — trios-doctor CLI entry point
//! Orchestrates: DR-01 (check) → DR-02 (heal) → DR-03 (report)
//! NO business logic here — all logic in Silver rings.

use trios_doctor_dr01::Doctor;
use trios_doctor_dr03::Reporter;

fn main() {
    let workspace = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".".to_string());

    let doctor = Doctor::new(&workspace);
    let diagnosis = doctor.run_all();

    Reporter::print_text(&diagnosis);

    let all_green = diagnosis
        .checks
        .iter()
        .all(|c| c.status == trios_doctor_dr00::CheckStatus::Green);

    if !all_green {
        std::process::exit(1);
    }
}
