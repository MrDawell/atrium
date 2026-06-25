use tokio::process::Command;

pub struct VerificationResult {
    pub is_valid: bool,
    pub compile_errors: Vec<String>,
    pub test_output: String,
    pub linter_output: String,
}

pub struct PatchVerifier {}

impl PatchVerifier {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn verify_patch(
        &self,
        patch_content: &str,
    ) -> Result<VerificationResult, Box<dyn std::error::Error + Send + Sync>> {
        println!("Verifying patch content (length: {} bytes)...", patch_content.len());

        let mut compile_errors = Vec::new();
        let mut is_valid = true;

        // 1. Run cargo check
        let check_child = Command::new("cargo")
            .arg("check")
            .output()
            .await;

        let check_success = match check_child {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if !output.status.success() {
                    is_valid = false;
                    compile_errors.push(format!("Cargo check failed:\n{}", stderr));
                }
                output.status.success()
            }
            Err(e) => {
                is_valid = false;
                let err_msg = format!("Failed to spawn cargo check: {}", e);
                compile_errors.push(err_msg.clone());
                false
            }
        };

        // 2. Run cargo test if check succeeded
        let mut test_output = String::new();
        if check_success {
            let test_child = Command::new("cargo")
                .arg("test")
                .output()
                .await;

            match test_child {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    test_output = format!("Stdout:\n{}\nStderr:\n{}", stdout, stderr);
                    if !output.status.success() {
                        is_valid = false;
                    }
                }
                Err(e) => {
                    is_valid = false;
                    test_output = format!("Failed to spawn cargo test: {}", e);
                }
            }
        } else {
            test_output = "Skipped tests because compilation check failed.".to_string();
        }

        // 3. Run cargo clippy for linting
        let mut linter_output = String::new();
        let clippy_child = Command::new("cargo")
            .arg("clippy")
            .output()
            .await;

        match clippy_child {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                linter_output = format!("Stdout:\n{}\nStderr:\n{}", stdout, stderr);
            }
            Err(e) => {
                linter_output = format!("Failed to spawn cargo clippy: {}", e);
            }
        }

        Ok(VerificationResult {
            is_valid,
            compile_errors,
            test_output,
            linter_output,
        })
    }
}

impl Default for PatchVerifier {
    fn default() -> Self {
        Self::new()
    }
}
