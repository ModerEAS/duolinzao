// 编译期注入版本串：显式 DLZ_VERSION 优先；否则抓 `git describe --tags --always`
// 原始串进 DLZ_GIT_DESCRIBE，由 src/version.rs 归一化为 Gitea 风格；
// 无 .git（Docker 默认忽略 .git）时用 GITHUB_SHA + Cargo 版本拼等价串，
// 本地无 git 时运行时回退 `{cargo_ver}+unknown`。
fn main() {
    println!("cargo:rerun-if-env-changed=DLZ_VERSION");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=.git/HEAD");

    if let Ok(v) = std::env::var("DLZ_VERSION") {
        if !v.trim().is_empty() {
            println!("cargo:rustc-env=DLZ_VERSION={}", v.trim());
            return;
        }
    }

    let cargo_ver =
        std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());

    if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
    {
        if output.status.success() {
            let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !raw.is_empty() {
                println!("cargo:rustc-env=DLZ_GIT_DESCRIBE={raw}");
                return;
            }
        }
    }

    // 无 git（Docker 无 .git / CI 浅克隆）：用 GITHUB_SHA 拼等价串的原始输入，
    // describe_to_version 会把纯 sha 归一化为 `{cargo_ver}+0-g{sha}`。
    if let Ok(sha) = std::env::var("GITHUB_SHA") {
        if sha.trim().len() >= 7 {
            let short: String = sha.trim().chars().take(7).collect();
            println!("cargo:rustc-env=DLZ_GIT_DESCRIBE={short}");
        }
    }
    let _ = cargo_ver;
}
