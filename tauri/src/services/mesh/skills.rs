//! Discover Claude Code "skills" from `~/.claude/skills/<slug>/SKILL.md`.
//! Returns slug + frontmatter (name, description) + markdown body for display
//! and later injection into an agent's prompt when a skill node is linked.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;

use crate::models::Skill;

fn skills_dir() -> io::Result<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".claude").join("skills"))
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "home directory not found"))
}

pub fn list_skills() -> io::Result<Vec<Skill>> {
    let dir = skills_dir()?;
    if !dir.exists() {
        return Ok(vec![]);
    }

    let entries = fs::read_dir(&dir)?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let slug = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let raw = match fs::read_to_string(&skill_md) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let (name, description, content) = parse_skill_md(&slug, &raw);
        out.push(Skill {
            slug,
            name,
            description,
            path: skill_md.to_string_lossy().to_string(),
            content,
        });
    }
    out.sort_by_key(|s| s.name.to_lowercase());
    Ok(out)
}

pub fn read_skill(slug: &str) -> io::Result<Skill> {
    let dir = skills_dir()?;
    let path = dir.join(slug).join("SKILL.md");
    if !path.exists() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("skill '{slug}' not found"),
        ));
    }
    let raw = fs::read_to_string(&path)?;
    let (name, description, content) = parse_skill_md(slug, &raw);
    Ok(Skill {
        slug: slug.to_string(),
        name,
        description,
        path: path.to_string_lossy().to_string(),
        content,
    })
}

/// Minimal YAML frontmatter parser: extracts `name:` and `description:` values
/// from a leading `---` block. Rest of the document is returned as content.
fn parse_skill_md(slug: &str, raw: &str) -> (String, String, String) {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return (slug.to_string(), String::new(), raw.to_string());
    }
    let after_first = &trimmed[3..];
    let end_idx = match after_first.find("\n---") {
        Some(i) => i,
        None => return (slug.to_string(), String::new(), raw.to_string()),
    };
    let frontmatter = &after_first[..end_idx];
    let body_start = end_idx + 4; // skip "\n---"
    let body = after_first[body_start..]
        .trim_start_matches('\n')
        .to_string();

    let mut name = slug.to_string();
    let mut description = String::new();
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name:") {
            name = unquote(rest.trim()).to_string();
        } else if let Some(rest) = line.strip_prefix("description:") {
            description = unquote(rest.trim()).to_string();
        }
    }
    (name, description, body)
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skill_md_extracts_name_and_description() {
        let raw =
            "---\nname: my-skill\ndescription: does cool stuff\n---\n\n# Body\n\ncontent here";
        let (name, desc, body) = parse_skill_md("fallback-slug", raw);
        assert_eq!(name, "my-skill");
        assert_eq!(desc, "does cool stuff");
        assert_eq!(body, "# Body\n\ncontent here");
    }

    #[test]
    fn parse_skill_md_handles_quoted_values() {
        let raw = "---\nname: \"quoted name\"\ndescription: 'single quoted'\n---\nbody";
        let (name, desc, _) = parse_skill_md("slug", raw);
        assert_eq!(name, "quoted name");
        assert_eq!(desc, "single quoted");
    }

    #[test]
    fn parse_skill_md_falls_back_to_slug_when_no_frontmatter() {
        let raw = "no frontmatter here, just markdown";
        let (name, desc, body) = parse_skill_md("my-slug", raw);
        assert_eq!(name, "my-slug");
        assert_eq!(desc, "");
        assert_eq!(body, raw);
    }

    #[test]
    fn parse_skill_md_falls_back_when_frontmatter_unterminated() {
        let raw = "---\nname: x\ndescription: y\n(no closing ---)";
        let (name, desc, body) = parse_skill_md("slug", raw);
        assert_eq!(name, "slug");
        assert_eq!(desc, "");
        assert_eq!(body, raw);
    }

    #[test]
    fn parse_skill_md_ignores_unknown_keys() {
        let raw = "---\nname: ok\nfoo: bar\nbaz: qux\n---\nbody";
        let (name, desc, _) = parse_skill_md("slug", raw);
        assert_eq!(name, "ok");
        assert_eq!(desc, "");
    }

    #[test]
    fn unquote_strips_matching_quotes() {
        assert_eq!(unquote("\"hi\""), "hi");
        assert_eq!(unquote("'hi'"), "hi");
        assert_eq!(unquote("hi"), "hi");
        assert_eq!(unquote("\""), "\"");
    }

    #[test]
    fn list_skills_returns_empty_when_dir_missing() {
        // Cannot easily mock skills_dir(); just verify the function does not panic
        // when the (likely missing) directory does not exist on the test machine.
        let _ = list_skills();
    }
}
