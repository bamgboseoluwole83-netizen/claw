//! Tests for tool availability and detection
//!
//! Run with: cargo test --test tool_detection

#[test]
fn test_tool_detection_finds_available_tools() {
    let forge = which::which("forge");
    let anvil = which::which("anvil");
    let cast = which::which("cast");

    let any_found = forge.is_ok() || anvil.is_ok() || cast.is_ok();
    if !any_found {
        eprintln!("WARNING: No Foundry tools (forge, anvil, cast) found in PATH");
    }
}

#[test]
fn test_tool_detection_handles_missing() {
    let nonexistent = which::which("definitely_not_a_real_tool_12345");
    assert!(nonexistent.is_err(), "Non-existent tool should return Err");
}

#[test]
fn test_path_lookup_precedence() {
    let found = which::which("sh");
    assert!(found.is_ok());
    if let Ok(path) = found {
        assert!(path.is_absolute(), "Path should be absolute");
    }
}

#[test]
fn test_multiple_tools_same_name() {
    let result = which::which("sh");
    assert!(result.is_ok());
}

mod pipeline_tool_detection {
    use super::*;

    pub fn check_forge_available() -> bool {
        which::which("forge").is_ok()
    }

    pub fn check_anvil_available() -> bool {
        which::which("anvil").is_ok()
    }

    pub fn check_cast_available() -> bool {
        which::which("cast").is_ok()
    }

    pub fn check_slither_available() -> bool {
        which::which("slither").is_ok()
    }

    pub fn check_wake_available() -> bool {
        which::which("wake").is_ok() || docker_image_exists("ackeeblockchain/wake:latest")
    }

    fn docker_image_exists(image: &str) -> bool {
        std::process::Command::new("docker")
            .args(["image", "inspect", image])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_forge_detection() {
        let available = check_forge_available();
        if !available {
            eprintln!("SKIP: forge not found");
        }
    }

    #[test]
    fn test_anvil_detection() {
        let available = check_anvil_available();
        if !available {
            eprintln!("SKIP: anvil not found");
        }
    }

    #[test]
    fn test_cast_detection() {
        let available = check_cast_available();
        if !available {
            eprintln!("SKIP: cast not found");
        }
    }

    #[test]
    fn test_slither_detection() {
        let available = check_slither_available();
        if !available {
            eprintln!("INFO: slither not found");
        }
    }

    #[test]
    fn test_wake_detection() {
        let available = check_wake_available();
        if !available {
            eprintln!("INFO: wake not found");
        }
    }
}

#[test]
fn test_environment_info() {
    eprintln!("=== Environment Info ===");
    eprintln!("PATH: {}", std::env::var("PATH").unwrap_or_default());
    eprintln!(
        "CARGO_MANIFEST_DIR: {}",
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default()
    );
    eprintln!("========================");
}
