use std::{
    hash::{Hash, Hasher},
    ops::Deref,
};

use regex::Regex;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};

/// Newtype wrapper for [`Regex`] enabling serde deserialization and hashing by
/// pattern.
#[serde_as]
#[derive(Debug, Clone, Deserialize,)]
#[serde(transparent)]
pub struct RegexCfg(#[serde_as(as = "DisplayFromStr")] pub Regex,);

impl PartialEq for RegexCfg
{
    fn eq(&self, other: &Self,) -> bool
    {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for RegexCfg {}

impl Hash for RegexCfg
{
    fn hash<H: Hasher,>(&self, state: &mut H,)
    {
        self.0.as_str().hash(state,);
    }
}

impl Deref for RegexCfg
{
    type Target = Regex;

    fn deref(&self,) -> &Self::Target
    {
        &self.0
    }
}

#[cfg(test)]
mod tests
{
    use serde::de::value::{Error as DeError, StrDeserializer};

    use super::*;

    #[test]
    fn regex_cfg_uses_pattern_for_equality()
    {
        let lhs = RegexCfg::deserialize(StrDeserializer::<DeError,>::new("foo",),).expect("lhs",);
        let rhs = RegexCfg::deserialize(StrDeserializer::<DeError,>::new("foo",),).expect("rhs",);
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn regex_cfg_hashes_by_pattern()
    {
        use std::collections::hash_map::DefaultHasher;

        let regex =
            RegexCfg::deserialize(StrDeserializer::<DeError,>::new("foo",),).expect("regex",);
        let mut hasher_a = DefaultHasher::new();
        regex.hash(&mut hasher_a,);

        let mut hasher_b = DefaultHasher::new();
        regex.hash(&mut hasher_b,);

        assert_eq!(hasher_a.finish(), hasher_b.finish());
    }
}
