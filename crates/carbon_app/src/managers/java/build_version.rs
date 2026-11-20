//! Numeric ordering of Java build versions, so that `17.0.21.0.3` sorts below
//! `17.0.21.1` and `1.8.0_492` equals `8.0.492`.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct JavaBuildVersion(Vec<u32>);

impl JavaBuildVersion {
    pub fn parse(s: &str) -> Option<Self> {
        let core = s.split(['-', '+']).next().unwrap_or("");

        if let Some((major, update)) = core.split_once('u') {
            if let (Ok(major), Ok(update)) = (major.parse::<u32>(), update.parse::<u32>()) {
                return Some(Self(vec![major, 0, update]));
            }
        }

        let mut components = core
            .split(['.', '_'])
            .map(|part| part.parse::<u32>().ok())
            .collect::<Option<Vec<u32>>>()?;

        if components.len() >= 3 && components[0] == 1 {
            components.remove(0);
        }

        Some(Self(components))
    }

    pub fn from_components(c: &[u32]) -> Self {
        Self(c.to_vec())
    }

    pub fn major(&self) -> u32 {
        self.0.first().copied().unwrap_or(0)
    }

    pub fn components(&self) -> &[u32] {
        &self.0
    }

    fn trimmed(&self) -> &[u32] {
        let end = self.0.iter().rposition(|c| *c != 0).map_or(0, |i| i + 1);
        &self.0[..end]
    }
}

impl PartialEq for JavaBuildVersion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for JavaBuildVersion {}

impl PartialOrd for JavaBuildVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JavaBuildVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        // Slice ordering compares element by element, and a shorter prefix
        // sorts first, so trimming trailing zeros makes a missing component
        // count as 0.
        self.trimmed().cmp(other.trimmed())
    }
}

impl Hash for JavaBuildVersion {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.trimmed().hash(state);
    }
}

impl fmt::Display for JavaBuildVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, c) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            write!(f, "{c}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> JavaBuildVersion {
        JavaBuildVersion::parse(s).unwrap()
    }

    #[test]
    fn orders_fourth_and_fifth_components() {
        assert!(v("17.0.21.0.3") < v("17.0.21.1"));
        assert!(v("17.0.21") < v("17.0.21.0.3"));
        assert!(v("17.0.20.1") < v("17.0.21"));
    }

    #[test]
    fn trailing_zeros_are_equal() {
        assert_eq!(v("17"), v("17.0.0"));
    }

    #[test]
    fn normalises_legacy_java8() {
        assert_eq!(
            v("1.8.0_492"),
            JavaBuildVersion::from_components(&[8, 0, 492])
        );
        assert_eq!(v("1.8.0_492").major(), 8);
    }

    #[test]
    fn ignores_suffixes() {
        assert_eq!(v("21.0.13.1+2-LTS"), v("21.0.13.1"));
        assert_eq!(v("25-ea"), v("25"));
    }

    #[test]
    fn normalises_mojang_legacy_names() {
        assert_eq!(v("8u202"), JavaBuildVersion::from_components(&[8, 0, 202]));
        assert_eq!(
            v("8u51-cacert462b08"),
            JavaBuildVersion::from_components(&[8, 0, 51])
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(JavaBuildVersion::parse("").is_none());
        assert!(JavaBuildVersion::parse("abc").is_none());
    }
}
