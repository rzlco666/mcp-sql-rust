/// Redact password from database URLs in error messages.
pub fn redact_url(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let scheme = &url[..scheme_end + 3];
        let rest = &url[scheme_end + 3..];
        if let Some(at) = rest.find('@') {
            let host_part = &rest[at..];
            if let Some(colon) = rest[..at].find(':') {
                let user = &rest[..colon];
                return format!("{scheme}{user}:***{host_part}");
            }
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_mysql_password() {
        let url = "mysql://root:secret@127.0.0.1:3306/mydb";
        assert_eq!(
            redact_url(url),
            "mysql://root:***@127.0.0.1:3306/mydb"
        );
    }
}
