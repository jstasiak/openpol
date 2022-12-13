use std::process::Command;

fn main() -> Result<(), String> {
    let output = Command::new("git")
        .args(["describe", "--dirty"])
        .output()
        .map_err(|e| e.to_string())?;
    let git_description = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    println!("cargo:rustc-env=GIT_DESCRIPTION={git_description}");
    Ok(())
}
