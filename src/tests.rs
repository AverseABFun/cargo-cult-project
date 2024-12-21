use super::*;
use proptest::prelude::*;

#[test]
fn format_from_string() {
    // this covers all valid cases but not invalid cases
    for ele in FORMATS {
        assert!(Format::from_str(ele).is_ok());
        assert!(Format::from_string(ele.to_string() + "-if-available").is_ok());
        assert!(Format::from_string(ele.to_string() + "-only").is_ok());

        assert_eq!(
            Format::from_str_no_err(ele),
            Format {
                format: ele.to_string(),
                suffix: Suffix::IfAvailable
            }
        );
        assert_eq!(
            Format::from_str(ele).unwrap(),
            Format {
                format: ele.to_string(),
                suffix: Suffix::IfAvailable
            }
        );

        assert_eq!(
            Format::from_string_no_err(ele.to_string() + "-if-available"),
            Format {
                format: ele.to_string(),
                suffix: Suffix::IfAvailable
            }
        );
        assert_eq!(
            Format::from_string(ele.to_string() + "-if-available").unwrap(),
            Format {
                format: ele.to_string(),
                suffix: Suffix::IfAvailable
            }
        );

        assert_eq!(
            Format::from_string_no_err(ele.to_string() + "-only"),
            Format {
                format: ele.to_string(),
                suffix: Suffix::Only
            }
        );
        assert_eq!(
            Format::from_string(ele.to_string() + "-only").unwrap(),
            Format {
                format: ele.to_string(),
                suffix: Suffix::Only
            }
        );
    }
}

proptest! {
    #[test]
    fn format_from_string_handles_utf8(s in "\\PC*") {
        Format::from_string_no_err(s.clone());
        let _ = Format::from_string(s);
    }
}
