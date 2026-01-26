use super::error::PluginError;

/// Parsed git URL components.
#[derive(Debug, Clone)]
pub struct GitSource {
    /// The host, e.g., "github.com"
    pub host: String,
    /// The repository owner, e.g., "anthropics"
    pub owner: String,
    /// The repository name, e.g., "claude-code"
    pub repo: String,
    /// The original URL
    pub url: String,
}

impl GitSource {
    /// Parse a git URL (HTTPS or SSH format).
    ///
    /// Supported formats:
    /// - `owner/repo` (shorthand, defaults to GitHub)
    /// - `https://github.com/owner/repo.git`
    /// - `https://github.com/owner/repo`
    /// - `git@github.com:owner/repo.git`
    /// - `git@github.com:owner/repo`
    pub fn parse(url: &str) -> Result<Self, PluginError> {
        let url_trimmed = url.trim();

        // Try shorthand format: owner/repo (defaults to GitHub)
        if !url_trimmed.contains("://") && !url_trimmed.starts_with("git@") {
            if let Some((owner, repo)) = url_trimmed.split_once('/') {
                if !owner.is_empty() && !repo.is_empty() && !repo.contains('/') {
                    let repo = repo.strip_suffix(".git").unwrap_or(repo);
                    return Ok(Self {
                        host: "github.com".to_string(),
                        owner: owner.to_string(),
                        repo: repo.to_string(),
                        url: format!("https://github.com/{}/{}", owner, repo),
                    });
                }
            }
        }

        // Try HTTPS format: https://github.com/owner/repo.git
        if let Some(rest) = url_trimmed.strip_prefix("https://") {
            return Self::parse_https(rest, url_trimmed);
        }

        // Try SSH format: git@github.com:owner/repo.git
        if let Some(rest) = url_trimmed.strip_prefix("git@") {
            return Self::parse_ssh(rest, url_trimmed);
        }

        Err(PluginError::InvalidUrl {
            url: url.to_string(),
        })
    }

    fn parse_https(rest: &str, original_url: &str) -> Result<Self, PluginError> {
        // rest = "github.com/owner/repo.git" or "github.com/owner/repo"
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(PluginError::InvalidUrl {
                url: original_url.to_string(),
            });
        }

        let host = parts[0].to_string();
        let path = parts[1];

        Self::parse_owner_repo(path, host, original_url)
    }

    fn parse_ssh(rest: &str, original_url: &str) -> Result<Self, PluginError> {
        // rest = "github.com:owner/repo.git" or "github.com:owner/repo"
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(PluginError::InvalidUrl {
                url: original_url.to_string(),
            });
        }

        let host = parts[0].to_string();
        let path = parts[1];

        Self::parse_owner_repo(path, host, original_url)
    }

    fn parse_owner_repo(
        path: &str,
        host: String,
        original_url: &str,
    ) -> Result<Self, PluginError> {
        // path = "owner/repo.git" or "owner/repo"
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.splitn(2, '/').collect();

        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(PluginError::InvalidUrl {
                url: original_url.to_string(),
            });
        }

        Ok(Self {
            host,
            owner: parts[0].to_string(),
            repo: parts[1].to_string(),
            url: original_url.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_https_with_git_suffix() {
        let source = GitSource::parse("https://github.com/anthropics/claude-code.git").unwrap();
        assert_eq!(source.host, "github.com");
        assert_eq!(source.owner, "anthropics");
        assert_eq!(source.repo, "claude-code");
    }

    #[test]
    fn test_parse_https_without_git_suffix() {
        let source = GitSource::parse("https://github.com/anthropics/claude-code").unwrap();
        assert_eq!(source.host, "github.com");
        assert_eq!(source.owner, "anthropics");
        assert_eq!(source.repo, "claude-code");
    }

    #[test]
    fn test_parse_ssh_with_git_suffix() {
        let source = GitSource::parse("git@github.com:anthropics/claude-code.git").unwrap();
        assert_eq!(source.host, "github.com");
        assert_eq!(source.owner, "anthropics");
        assert_eq!(source.repo, "claude-code");
    }

    #[test]
    fn test_parse_ssh_without_git_suffix() {
        let source = GitSource::parse("git@github.com:anthropics/claude-code").unwrap();
        assert_eq!(source.host, "github.com");
        assert_eq!(source.owner, "anthropics");
        assert_eq!(source.repo, "claude-code");
    }

    #[test]
    fn test_parse_gitlab() {
        let source = GitSource::parse("https://gitlab.com/user/project.git").unwrap();
        assert_eq!(source.host, "gitlab.com");
        assert_eq!(source.owner, "user");
        assert_eq!(source.repo, "project");
    }

    #[test]
    fn test_parse_invalid_url() {
        assert!(GitSource::parse("not-a-url").is_err());
        assert!(GitSource::parse("https://github.com").is_err());
        assert!(GitSource::parse("https://github.com/").is_err());
        assert!(GitSource::parse("https://github.com/owner").is_err());
    }

    #[test]
    fn test_parse_shorthand() {
        let source = GitSource::parse("anthropics/claude-code").unwrap();
        assert_eq!(source.host, "github.com");
        assert_eq!(source.owner, "anthropics");
        assert_eq!(source.repo, "claude-code");
        assert_eq!(source.url, "https://github.com/anthropics/claude-code");
    }

    #[test]
    fn test_parse_shorthand_with_git_suffix() {
        let source = GitSource::parse("anthropics/claude-code.git").unwrap();
        assert_eq!(source.host, "github.com");
        assert_eq!(source.owner, "anthropics");
        assert_eq!(source.repo, "claude-code");
        assert_eq!(source.url, "https://github.com/anthropics/claude-code");
    }

    #[test]
    fn test_parse_shorthand_invalid() {
        assert!(GitSource::parse("owner").is_err());
        assert!(GitSource::parse("/repo").is_err());
        assert!(GitSource::parse("owner/").is_err());
    }
}
