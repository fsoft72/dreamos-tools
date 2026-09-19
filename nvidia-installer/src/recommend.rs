use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecommendationSource {
    Detected,
    FallbackDefault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recommendation {
    pub package: String,
    pub source: RecommendationSource,
}

const FALLBACK_PACKAGE: &str = "nvidia-driver";

/// Extracts the package name `nvidia-detect` recommends. Its output puts
/// the package name alone on the line right after "It is recommended to
/// install the" (indented), so this scans for that anchor line rather
/// than a single-line regex, since the package name's own line has no
/// other distinguishing marker.
pub fn parse_nvidia_detect(output: &str) -> Option<String> {
    let mut lines = output.lines();
    while let Some(line) = lines.next() {
        if line.trim_end() == "It is recommended to install the" {
            let pkg = lines.next()?.trim();
            if !pkg.is_empty() {
                return Some(pkg.to_string());
            }
        }
    }
    None
}

pub fn recommend_package() -> Recommendation {
    let output = Command::new("nvidia-detect").output();
    let stdout = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        _ => {
            return Recommendation {
                package: FALLBACK_PACKAGE.to_string(),
                source: RecommendationSource::FallbackDefault,
            }
        }
    };

    match parse_nvidia_detect(&stdout) {
        Some(package) => Recommendation { package, source: RecommendationSource::Detected },
        None => Recommendation {
            package: FALLBACK_PACKAGE.to_string(),
            source: RecommendationSource::FallbackDefault,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_recommendation() {
        let fixture = include_str!("../tests/fixtures/nvidia_detect_found.txt");
        assert_eq!(parse_nvidia_detect(fixture), Some("nvidia-driver".to_string()));
    }

    #[test]
    fn parses_legacy_recommendation() {
        let fixture = include_str!("../tests/fixtures/nvidia_detect_legacy.txt");
        assert_eq!(
            parse_nvidia_detect(fixture),
            Some("nvidia-tesla-470-driver".to_string())
        );
    }

    #[test]
    fn unparseable_output_returns_none() {
        assert_eq!(parse_nvidia_detect("no useful output here\n"), None);
    }
}
