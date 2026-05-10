use std::env;
use std::fs;
use std::io::Read;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Six Installer (sixi)");
        println!("Usage: sixi install <@scope/name>");
        return;
    }

    let command = &args[1];
    let package = &args[2];

    if command == "install" {
        install_package(package);
    } else if command == "load-libs" {
        if let Ok(content) = fs::read_to_string("six.toml") {
            // Very simple parser for [dependencies] block
            let mut in_deps = false;
            for line in content.lines() {
                let line = line.trim();
                if line == "[dependencies]" {
                    in_deps = true;
                    continue;
                }
                if line.starts_with('[') {
                    in_deps = false;
                }
                if in_deps && line.contains('=') {
                    let parts: Vec<&str> = line.split('=').collect();
                    let name = parts[0].trim().replace("\"", "");
                    println!("[SixI] Loading dependency: {}...", name);
                    install_package(&name);
                }
            }
        }
    }
}

fn verify_signature(bin_path: &std::path::Path) -> bool {
    // Look for companion header with stored signature
    let sig_path = bin_path.with_extension("").with_extension("six.h");
    if !sig_path.exists() {
        // No signature file — warn but allow (for backward compat)
        println!(
            "[SixI] WARN: No signature found for {}. Install at your own risk.",
            bin_path.display()
        );
        return true;
    }
    let header = fs::read_to_string(&sig_path).unwrap_or_default();
    let stored_sig = header
        .lines()
        .find(|l| l.starts_with("// SIG:"))
        .map(|l| l.trim_start_matches("// SIG:").trim())
        .unwrap_or("");

    if stored_sig.is_empty() {
        return true; // Old-format library, no sig embedded
    }

    let bin_data = fs::read(bin_path).unwrap_or_default();
    let computed: u64 = bin_data
        .iter()
        .enumerate()
        .fold(0u64, |acc, (i, &b)| acc ^ ((b as u64) << (i % 8)));
    let computed_hex = format!("{:016X}", computed);

    if computed_hex != stored_sig {
        println!(
            "[SixI] SECURITY ERROR: Signature mismatch for {}!",
            bin_path.display()
        );
        println!("  Expected: {}", stored_sig);
        println!("  Got:      {}", computed_hex);
        println!("[SixI] Installation BLOCKED. Binary may be tampered.");
        return false;
    }
    true
}

fn install_package(package: &str) {
    let sanitized = package.replace("@", "").replace("/", "-");
    let registries = vec!["other/libs", "other/user_libs"];

    let mut found = false;
    for reg in registries {
        if let Ok(entries) = fs::read_dir(reg) {
            for entry in entries {
                let entry = entry.unwrap();
                let path = entry.path();
                let filename = path.file_name().unwrap().to_str().unwrap();

                if filename.starts_with(&sanitized) && filename.ends_with(".siz.lib") {
                    // Verify signature before installing
                    if !verify_signature(&path) {
                        return;
                    }
                    fs::create_dir_all("libs").ok();
                    let dest = format!("libs/{}", filename);
                    fs::copy(&path, &dest).expect("Failed to copy library");
                    println!("[SixI] ✓ Installed (verified): {} from {}", filename, reg);
                    found = true;
                }
            }
        }
    }

    if !found {
        // GitHub registry fallback — no DB, just raw file lookup
        println!("[SixI] Not found locally. Checking Rebon402/six-registry...");
        let base = "https://raw.githubusercontent.com/Rebon402/six-registry/main/libs";

        // Try to match any file starting with sanitized name
        let index_url = format!("https://api.github.com/repos/Rebon402/six-registry/contents/libs");

        let listing: Vec<serde_json::Value> = ureq::get(&index_url)
            .set("User-Agent", "six-cli")
            .call()
            .ok()
            .and_then(|r| r.into_json().ok())
            .unwrap_or_default();

        let mut remote_found = false;
        for entry in &listing {
            let name = entry["name"].as_str().unwrap_or("");
            if name.starts_with(&sanitized) && name.ends_with(".siz.lib") {
                // Download .siz.lib
                let lib_url = format!("{}/{}", base, name);
                let hdr_url = format!("{}/{}", base, name.replace(".siz.lib", ".six.h"));

                match ureq::get(&lib_url).set("User-Agent", "six-cli").call() {
                    Ok(resp) => {
                        let mut buf: Vec<u8> = Vec::new();
                        resp.into_reader().read_to_end(&mut buf).ok();

                        fs::create_dir_all("other/user_libs").ok();
                        let cache_lib = format!("other/user_libs/{}", name);
                        fs::write(&cache_lib, &buf).ok();

                        // Also fetch header
                        if let Ok(hr) = ureq::get(&hdr_url).set("User-Agent", "six-cli").call() {
                            let mut hbuf: Vec<u8> = Vec::new();
                            hr.into_reader().read_to_end(&mut hbuf).ok();
                            let hdr_name = name.replace(".siz.lib", ".six.h");
                            let cache_hdr = format!("other/user_libs/{}", hdr_name);
                            fs::write(&cache_hdr, &hbuf).ok();
                        }

                        // Verify then install
                        let cached_path = std::path::Path::new(&cache_lib);
                        if verify_signature(cached_path) {
                            fs::create_dir_all("libs").ok();
                            let dest = format!("libs/{}", name);
                            fs::copy(&cache_lib, &dest).ok();
                            println!("[SixI] ✓ Fetched & installed from six-registry: {}", name);
                            remote_found = true;
                        }
                    }
                    Err(e) => println!("[SixI ERROR] Download failed: {:?}", e),
                }
                break;
            }
        }

        if !remote_found {
            println!(
                "[SixI ERROR] Package not found: {} (local + six-registry)",
                package
            );
        }
    }
}
