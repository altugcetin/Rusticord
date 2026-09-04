pub const APPLICATION_IDENTIFIER: &str = "ist.alchm.rusticord";

#[cfg(test)]
mod tests {
    use super::APPLICATION_IDENTIFIER;

    #[test]
    fn identifier_has_three_dns_labels() {
        let mut labels = APPLICATION_IDENTIFIER.split('.');
        assert_eq!(labels.next(), Some("ist"));
        assert_eq!(labels.next(), Some("alchm"));
        assert_eq!(labels.next(), Some("rusticord"));
        assert_eq!(labels.next(), None);
    }
}
