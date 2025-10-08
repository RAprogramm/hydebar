use crate::config;

pub(crate) fn is_output_requested(name: Option<&str,>, outputs: &config::Outputs,) -> bool
{
    match outputs {
        config::Outputs::All => true,
        config::Outputs::Active => false,
        config::Outputs::Targets(request_outputs,) => {
            request_outputs.iter().any(|output| Some(output.as_str(),) == name,)
        }
    }
}

#[cfg(test)]
mod tests
{
    use hydebar_proto::config::Outputs;

    use super::*;

    #[test]
    fn targets_match_name()
    {
        let requested = Outputs::Targets(vec!["DP-1".into(), "HDMI-A-1".into()],);
        assert!(is_output_requested(Some("DP-1"), &requested));
        assert!(!is_output_requested(Some("eDP-1"), &requested));
    }

    #[test]
    fn all_accepts_anything()
    {
        assert!(is_output_requested(Some("foo"), &Outputs::All));
        assert!(is_output_requested(None, &Outputs::All));
    }

    #[test]
    fn active_rejects_all()
    {
        assert!(!is_output_requested(Some("foo"), &Outputs::Active));
        assert!(!is_output_requested(None, &Outputs::Active));
    }
}
