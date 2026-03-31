use std::process::Command;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    fn init_git_repo(dir: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
        let _ = Command::new("git")
            .args(&["init"])
            .current_dir(dir.path())
            .output()?;
        let _ = Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(dir.path())
            .output();
        let _ = Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(dir.path())
            .output();
        Ok(())
    }

    fn commit_file(dir: &TempDir, filename: &str, content: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = dir.path().join(filename);
        std::fs::write(&path, content)?;
        let _ = Command::new("git").args(&["add", filename]).current_dir(dir.path()).output();
        let _ = Command::new("git").args(&["-c", "core.autocrlf=false", "commit", "-m", message]).current_dir(dir.path()).output();
        Ok(())
    }

    #[test]
    fn test_binary_produces_markdown() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        init_git_repo(&dir)?;

        // Create a small Rust project
        commit_file(&dir, "README.md", "# Test Project\n\nA test repo for tutorial generation.", "Add README")?;
        commit_file(&dir, "main.rs", "fn main() {\n    println!(\"Hello, world!\");\n}", "Add main")?;
        commit_file(&dir, "lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }", "Add lib")?;
        commit_file(&dir, "Cargo.toml", "[package]\nname = \"test-proj\"\nversion = \"0.1.0\"\nedition = \"2021\"", "Add manifest")?;

        let output_file = dir.path().join("tutorial.md");

        // Build and run
        let status = Command::new("target/release/tutorial-generator")
            .arg(dir.path())
            .arg("--output").arg(&output_file)
            .arg("--title").arg("Test Tutorial")
            .arg("--max-commits").arg("10")
            .status()?;

        assert!(status.success(), "Binary should exit with success");
        assert!(output_file.exists(), "Output file should exist");

        let content = std::fs::read_to_string(&output_file)?;
        assert!(content.contains("# Test Tutorial"), "Should have title");
        assert!(content.contains("## Introduction"), "Should have Introduction section");
        assert!(content.contains("## Conclusion"), "Should have Conclusion section");
        assert!(content.contains("**Repository:**"), "Should have repository info");
        assert!(content.contains("**Languages:**"), "Should have languages info");
        assert!(content.contains("**Total commits:**"), "Should show commit count");

        Ok(())
    }

    #[test]
    fn test_binary_json_output() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        init_git_repo(&dir)?;

        commit_file(&dir, "README.md", "# JSON Test", "Add README")?;

        let output_file = dir.path().join("tutorial.json");

        let status = Command::new("target/release/tutorial-generator")
            .arg(dir.path())
            .arg("--output").arg(&output_file)
            .arg("--format").arg("json")
            .arg("--max-commits").arg("5")
            .status()?;

        assert!(status.success());
        assert!(output_file.exists());

        let content = std::fs::read_to_string(&output_file)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        assert!(json.get("title").is_some(), "Should have title field");
        assert!(json.get("sections").is_some(), "Should have sections field");
        assert!(json.get("repository").is_some(), "Should have repository field");

        Ok(())
    }

    #[test]
    #[ignore] // Skip network test by default
    fn test_clone_functionality() -> Result<(), Box<dyn std::error::Error>> {
        // This test is slow and requires network, so ignore by default
        let output_dir = TempDir::new()?;
        let status = Command::new("target/release/tutorial-generator")
            .arg("https://github.com/rust-lang/rust.git")
            .arg("--clone")
            .arg("--dir").arg(output_dir.path())
            .arg("--max-commits").arg("3")
            .status()?;
        assert!(status.success());
        Ok(())
    }

    #[test]
    fn test_help_output() -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new("target/release/tutorial-generator")
            .arg("--help")
            .output()?;
        let stdout = String::from_utf8(output.stdout)?;
        assert!(stdout.contains("USAGE:") || stdout.contains("Usage:"));
        assert!(stdout.contains("repo"), "Should show repo argument");
        assert!(stdout.contains("--output"), "Should show output option");
        assert!(stdout.contains("--title"), "Should show title option");
        Ok(())
    }
}
