use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: &str) -> Result<SubscriberEmail, String> {
        if s.validate_email() {
            Ok(Self(s.to_string()))
        } else {
            Err(format!("\"{}\" is not a valid email address", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};
    use rand::{rngs::StdRng, SeedableRng};

    use crate::domain::SubscriberEmail;

    #[derive(Clone, Debug)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        assert_err!(SubscriberEmail::parse(""));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        assert_err!(SubscriberEmail::parse("testdomain.com"));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        assert_err!(SubscriberEmail::parse("@domain.com"));
    }

    quickcheck! {
        fn valid_mail_is_ok(valid_email: ValidEmailFixture) -> bool {
            SubscriberEmail::parse(&valid_email.0).is_ok()
        }
    }
}
