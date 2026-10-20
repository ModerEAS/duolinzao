//! Gitea 风格版本串：正好落在 `vX.Y.Z` 上显示 `X.Y.Z`；
//! 否则 `git describe --tags --always` 首个 `-` 换 `+` 并去开头 `v`
//!（如 `0.1.0+12-gc7d0119`）；无 tag 时只有短 sha，则拼
//! `{cargo_ver}+0-g{sha}`；无 git 时回退 `{cargo_ver}+unknown`。

/// 把 `git describe --tags --always` 原始输出归一化为页脚版本串。
pub fn describe_to_version(raw: &str, cargo_ver: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return format!("{cargo_ver}+unknown");
    }
    if s.contains('-') {
        // `v0.1.0-12-gc7d0119` → `0.1.0+12-gc7d0119`（只换第一个 `-`）。
        let once = s.replacen('-', "+", 1);
        return once.strip_prefix('v').unwrap_or(&once).to_string();
    }
    if let Some(tag) = s.strip_prefix('v') {
        // 精确落在 tag 上：`v0.1.0` → `0.1.0`（去掉 `v`，不附 commit）。
        return tag.to_string();
    }
    if s == cargo_ver {
        return s.to_string();
    }
    // 无 tag 时 `--always` 只给出短 sha：拼出与 Gitea 等价的串。
    format!("{cargo_ver}+0-g{s}")
}

/// 编译期注入的最终版本串：显式 `DLZ_VERSION` 优先（已是归一化串，原样用），
/// 否则用 `DLZ_GIT_DESCRIBE` 归一化，无 git 时回退 `CARGO_PKG_VERSION+unknown`。
pub fn app_version() -> String {
    if let Some(v) = option_env!("DLZ_VERSION") {
        let t = v.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    if let Some(raw) = option_env!("DLZ_GIT_DESCRIBE") {
        return describe_to_version(raw, env!("CARGO_PKG_VERSION"));
    }
    format!("{}+unknown", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_tag_strips_v() {
        assert_eq!(describe_to_version("v0.1.0", "0.1.0"), "0.1.0");
    }

    #[test]
    fn dev_build_replaces_first_dash_only() {
        assert_eq!(
            describe_to_version("v0.1.0-12-gc7d0119", "0.1.0"),
            "0.1.0+12-gc7d0119"
        );
    }

    #[test]
    fn bare_sha_gets_cargo_baseline() {
        assert_eq!(
            describe_to_version("c7d0119", "0.1.0"),
            "0.1.0+0-gc7d0119"
        );
    }

    #[test]
    fn empty_falls_back() {
        assert_eq!(describe_to_version("", "0.1.0"), "0.1.0+unknown");
    }
}
