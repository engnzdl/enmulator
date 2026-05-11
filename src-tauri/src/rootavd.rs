use std::path::PathBuf;
use std::process::Command;

/// Clone (or update) the rootAVD repository to the given directory.
pub fn clone_or_update(repo_dir: &PathBuf) -> Result<String, String> {
    if repo_dir.join(".git").exists() {
        // Already cloned — pull latest
        let output = Command::new("git")
            .args(["-C", repo_dir.to_str().unwrap_or("."), "pull", "--ff-only"])
            .output()
            .map_err(|e| format!("git pull error: {}", e))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        Ok("rootAVD updated successfully".into())
    } else {
        // Clone fresh
        std::fs::create_dir_all(repo_dir.parent().unwrap_or(repo_dir))
            .map_err(|e| format!("mkdir error: {}", e))?;
        let output = Command::new("git")
            .args([
                "clone",
                "https://gitlab.com/newbit/rootAVD.git",
                repo_dir.to_str().unwrap_or("rootAVD"),
            ])
            .output()
            .map_err(|e| format!("git clone error: {}", e))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        Ok("rootAVD cloned successfully".into())
    }
}

/// Find the ramdisk.img for a given API level and tag/abi.
/// Default: tag = "google_apis", abi = "x86_64"
pub fn find_ramdisk(
    sdk_path: &PathBuf,
    api_level: u8,
    tag: &str,
    abi: &str,
) -> Result<PathBuf, String> {
    let candidate = sdk_path
        .join("system-images")
        .join(format!("android-{}", api_level))
        .join(tag)
        .join(abi)
        .join("ramdisk.img");

    if candidate.exists() {
        Ok(candidate)
    } else {
        // Try to find any tag/abi combination for this API level
        let base = sdk_path
            .join("system-images")
            .join(format!("android-{}", api_level));

        if !base.exists() {
            return Err(format!(
                "No system images found for API {}. Install via SDK Manager first.",
                api_level
            ));
        }

        // Walk the directory tree looking for ramdisk.img
        let mut found: Option<PathBuf> = None;
        if let Ok(entries) = std::fs::read_dir(&base) {
            for tag_entry in entries.flatten() {
                if !tag_entry.path().is_dir() {
                    continue;
                }
                if let Ok(abi_entries) = std::fs::read_dir(tag_entry.path()) {
                    for abi_entry in abi_entries.flatten() {
                        let ramdisk = abi_entry.path().join("ramdisk.img");
                        if ramdisk.exists() {
                            found = Some(ramdisk);
                            break;
                        }
                    }
                }
                if found.is_some() {
                    break;
                }
            }
        }

        found.ok_or_else(|| {
            format!(
                "ramdisk.img not found for API {}. Expected at: {}",
                api_level,
                candidate.display()
            )
        })
    }
}

/// Run rootAVD on a device's ramdisk.img.
/// rootavd_dir: path to the cloned rootAVD repository.
/// ramdisk_path: path to the ramdisk.img to patch.
pub fn patch_ramdisk(rootavd_dir: &PathBuf, ramdisk_path: &PathBuf) -> Result<String, String> {
    let script = rootavd_dir.join("rootAVD.sh");

    if !script.exists() {
        return Err(format!(
            "rootAVD.sh not found at {}. Did you download rootAVD?",
            script.display()
        ));
    }

    let output = Command::new("bash")
        .arg(script.to_str().unwrap_or("rootAVD.sh"))
        .arg(ramdisk_path.to_str().unwrap_or(""))
        .current_dir(rootavd_dir)
        .output()
        .map_err(|e| format!("rootAVD execution error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("rootAVD failed:\n{}\n{}", stdout, stderr));
    }

    Ok(stdout)
}

/// High-level: download rootAVD (if needed) and root a device.
/// Returns a success message.
pub fn toggle_root(
    sdk_path: &PathBuf,
    api_level: u8,
    repo_dir: &PathBuf,
) -> Result<String, String> {
    // 1. Ensure rootAVD is available
    if !repo_dir.join("rootAVD.sh").exists() {
        clone_or_update(repo_dir)?;
    }

    // 2. Find ramdisk for this device
    let ramdisk = find_ramdisk(sdk_path, api_level, "google_apis", "x86_64")?;

    // 3. Back up original ramdisk if backup doesn't exist
    let backup = ramdisk.with_extension("img.bak");
    if !backup.exists() {
        std::fs::copy(&ramdisk, &backup)
            .map_err(|e| format!("Failed to back up ramdisk.img: {}", e))?;
    }

    // 4. Patch the ramdisk
    patch_ramdisk(repo_dir, &ramdisk)?;

    Ok(format!(
        "Device rooted successfully. Ramdisk patched: {}\nBackup saved to: {}\nRestart the device for changes to take effect.",
        ramdisk.display(),
        backup.display()
    ))
}

/// Restore original ramdisk from backup (unroot).
pub fn unroot(sdk_path: &PathBuf, api_level: u8) -> Result<String, String> {
    let ramdisk = find_ramdisk(sdk_path, api_level, "google_apis", "x86_64")?;
    let backup = ramdisk.with_extension("img.bak");

    if !backup.exists() {
        return Err("No backup found. Cannot unroot.".into());
    }

    std::fs::copy(&backup, &ramdisk)
        .map_err(|e| format!("Failed to restore ramdisk.img from backup: {}", e))?;

    Ok("Device unrooted. Original ramdisk restored. Restart the device for changes to take effect.".into())
}
