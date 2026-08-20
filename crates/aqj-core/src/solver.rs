use crate::error::{AqjError, Result};
use crate::metadata::PackageMetadata;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Represents a parsed version number for comparison (e.g. "2.12.1_1")
#[derive(Debug, Clone, Eq)]
pub struct Version {
    pub numbers: Vec<u64>,
    pub revision: u32,
    pub raw: String,
}

impl Version {
    pub fn parse(s: &str) -> Self {
        let (ver_part, rev) = if let Some((v, r)) = s.split_once('_') {
            (v, r.parse::<u32>().unwrap_or(1))
        } else {
            (s, 1)
        };

        let numbers: Vec<u64> = ver_part
            .split(|c: char| !c.is_numeric())
            .filter(|p| !p.is_empty())
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect();

        Self {
            numbers,
            revision: rev,
            raw: s.to_string(),
        }
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.numbers == other.numbers && self.revision == other.revision
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let max_len = self.numbers.len().max(other.numbers.len());
        for i in 0..max_len {
            let n1 = self.numbers.get(i).copied().unwrap_or(0);
            let n2 = other.numbers.get(i).copied().unwrap_or(0);
            match n1.cmp(&n2) {
                Ordering::Equal => continue,
                non_eq => return non_eq,
            }
        }
        self.revision.cmp(&other.revision)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// Comparison operator for version constraints
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionOperator {
    Any,
    Equal,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

/// A version constraint like ">=2.12.1"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionConstraint {
    pub op: VersionOperator,
    pub target: Option<Version>,
    pub raw: String,
}

impl VersionConstraint {
    pub fn any() -> Self {
        Self {
            op: VersionOperator::Any,
            target: None,
            raw: "*".to_string(),
        }
    }

    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.is_empty() || s == "*" {
            return Self::any();
        }

        let (op, ver_str) = if let Some(rest) = s.strip_prefix(">=") {
            (VersionOperator::GreaterThanOrEqual, rest)
        } else if let Some(rest) = s.strip_prefix("<=") {
            (VersionOperator::LessThanOrEqual, rest)
        } else if let Some(rest) = s.strip_prefix("==") {
            (VersionOperator::Equal, rest)
        } else if let Some(rest) = s.strip_prefix('=') {
            (VersionOperator::Equal, rest)
        } else if let Some(rest) = s.strip_prefix('>') {
            (VersionOperator::GreaterThan, rest)
        } else if let Some(rest) = s.strip_prefix('<') {
            (VersionOperator::LessThan, rest)
        } else {
            (VersionOperator::Equal, s)
        };

        let ver = Version::parse(ver_str.trim());
        Self {
            op,
            target: Some(ver),
            raw: s.to_string(),
        }
    }

    pub fn matches(&self, ver: &Version) -> bool {
        let target = match &self.target {
            Some(t) => t,
            None => return true,
        };

        match self.op {
            VersionOperator::Any => true,
            VersionOperator::Equal => ver == target,
            VersionOperator::GreaterThan => ver > target,
            VersionOperator::GreaterThanOrEqual => ver >= target,
            VersionOperator::LessThan => ver < target,
            VersionOperator::LessThanOrEqual => ver <= target,
        }
    }
}

/// A parsed dependency requirement, e.g. "hello>=2.12.1"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyReq {
    pub name: String,
    pub constraint: VersionConstraint,
}

impl DependencyReq {
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        // Find operator position
        let op_pos = s.find(|c| c == '=' || c == '>' || c == '<');
        if let Some(pos) = op_pos {
            let name = s[..pos].trim().to_string();
            let constraint_str = s[pos..].trim();
            Self {
                name,
                constraint: VersionConstraint::parse(constraint_str),
            }
        } else {
            Self {
                name: s.to_string(),
                constraint: VersionConstraint::any(),
            }
        }
    }
}

/// Dependency solver and topological sorter
pub struct DependencySolver;

impl DependencySolver {
    /// Resolves dependencies for a target package (or set of targets) given a universe of available packages.
    /// Returns an ordered list of packages to install/build (topologically sorted: dependencies first).
    pub fn resolve(
        targets: &[&str],
        available: &HashMap<String, PackageMetadata>,
    ) -> Result<Vec<PackageMetadata>> {
        let mut resolved_order: Vec<PackageMetadata> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut visiting: Vec<String> = Vec::new();

        for &target in targets {
            Self::visit(target, available, &mut visited, &mut visiting, &mut resolved_order)?;
        }

        Ok(resolved_order)
    }

    fn visit(
        pkg_name: &str,
        available: &HashMap<String, PackageMetadata>,
        visited: &mut HashSet<String>,
        visiting: &mut Vec<String>,
        resolved_order: &mut Vec<PackageMetadata>,
    ) -> Result<()> {
        if visited.contains(pkg_name) {
            return Ok(());
        }

        if visiting.contains(&pkg_name.to_string()) {
            visiting.push(pkg_name.to_string());
            return Err(AqjError::CircularDependency {
                chain: visiting.join(" -> "),
            });
        }

        let pkg = available.get(pkg_name).ok_or_else(|| AqjError::UnsatisfiedDependency {
            package: visiting.last().cloned().unwrap_or_else(|| "root".to_string()),
            dependency: pkg_name.to_string(),
        })?;

        visiting.push(pkg_name.to_string());

        for dep_str in &pkg.depends {
            let req = DependencyReq::parse(dep_str);
            
            // Check if dependency exists in available pool
            let dep_pkg = available.get(&req.name).ok_or_else(|| AqjError::UnsatisfiedDependency {
                package: pkg_name.to_string(),
                dependency: dep_str.clone(),
            })?;

            // Check version constraint
            let dep_version = Version::parse(&dep_pkg.version);
            if !req.constraint.matches(&dep_version) {
                return Err(AqjError::DependencyConflict {
                    package: pkg_name.to_string(),
                    dependency: req.name.clone(),
                    constraint: req.constraint.raw.clone(),
                    found: dep_pkg.version.clone(),
                });
            }

            Self::visit(&req.name, available, visited, visiting, resolved_order)?;
        }

        visiting.pop();
        visited.insert(pkg_name.to_string());
        resolved_order.push(pkg.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing_and_comparison() {
        let v1 = Version::parse("1.2.0");
        let v2 = Version::parse("1.10.0");
        let v3 = Version::parse("2.0.0");
        let v4 = Version::parse("1.2.0_2");

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v4);
        assert_eq!(v1, Version::parse("1.2.0_1"));
    }

    #[test]
    fn test_version_constraint_matching() {
        let ver = Version::parse("2.12.1");

        assert!(VersionConstraint::parse(">=2.0.0").matches(&ver));
        assert!(VersionConstraint::parse("<3.0.0").matches(&ver));
        assert!(VersionConstraint::parse("==2.12.1").matches(&ver));
        assert!(!VersionConstraint::parse(">3.0.0").matches(&ver));
        assert!(!VersionConstraint::parse("<=2.0.0").matches(&ver));
    }

    #[test]
    fn test_dependency_req_parsing() {
        let req1 = DependencyReq::parse("glibc>=2.33");
        assert_eq!(req1.name, "glibc");
        assert_eq!(req1.constraint.op, VersionOperator::GreaterThanOrEqual);

        let req2 = DependencyReq::parse("zlib");
        assert_eq!(req2.name, "zlib");
        assert_eq!(req2.constraint.op, VersionOperator::Any);
    }

    #[test]
    fn test_solver_topological_sort() {
        let mut available = HashMap::new();

        let pkg_c = PackageMetadata {
            name: "libc".to_string(),
            version: "2.33".to_string(),
            revision: 1,
            architecture: "x86_64".to_string(),
            summary: "C library".to_string(),
            license: "LGPL".to_string(),
            homepage: None,
            depends: vec![],
            build_date: 0,
        };

        let pkg_b = PackageMetadata {
            name: "zlib".to_string(),
            version: "1.2.11".to_string(),
            revision: 1,
            architecture: "x86_64".to_string(),
            summary: "Zlib".to_string(),
            license: "Zlib".to_string(),
            homepage: None,
            depends: vec!["libc>=2.30".to_string()],
            build_date: 0,
        };

        let pkg_a = PackageMetadata {
            name: "app".to_string(),
            version: "1.0.0".to_string(),
            revision: 1,
            architecture: "x86_64".to_string(),
            summary: "App".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            depends: vec!["zlib".to_string(), "libc".to_string()],
            build_date: 0,
        };

        available.insert("libc".to_string(), pkg_c);
        available.insert("zlib".to_string(), pkg_b);
        available.insert("app".to_string(), pkg_a);

        let resolved = DependencySolver::resolve(&["app"], &available).unwrap();
        let names: Vec<String> = resolved.into_iter().map(|p| p.name).collect();

        // Dependencies must come before app
        assert_eq!(names, vec!["libc", "zlib", "app"]);
    }

    #[test]
    fn test_solver_circular_dependency() {
        let mut available = HashMap::new();

        let pkg_a = PackageMetadata {
            name: "pkg-a".to_string(),
            version: "1.0".to_string(),
            revision: 1,
            architecture: "x86_64".to_string(),
            summary: "A".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            depends: vec!["pkg-b".to_string()],
            build_date: 0,
        };

        let pkg_b = PackageMetadata {
            name: "pkg-b".to_string(),
            version: "1.0".to_string(),
            revision: 1,
            architecture: "x86_64".to_string(),
            summary: "B".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            depends: vec!["pkg-a".to_string()],
            build_date: 0,
        };

        available.insert("pkg-a".to_string(), pkg_a);
        available.insert("pkg-b".to_string(), pkg_b);

        let result = DependencySolver::resolve(&["pkg-a"], &available);
        assert!(result.is_err());
        match result.unwrap_err() {
            AqjError::CircularDependency { chain } => {
                assert!(chain.contains("pkg-a -> pkg-b -> pkg-a"));
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }
}
