use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt;
use std::ops::Sub;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Eq, PartialEq)]
pub struct SignedDuration {
    pub duration: Duration,
    pub is_positive: bool,
}

impl SignedDuration {
    pub fn as_nanos(&self) -> i128 {
        let sign = if self.is_positive { 1 } else { -1 };

        sign * (self.duration.as_nanos() as i128)
    }

    pub fn from_nanos(nanos: i128) -> SignedDuration {
        let is_positive = nanos >= 0;

        SignedDuration {
            duration: Duration::from_nanos(nanos.abs() as u64),
            is_positive,
        }
    }
}

impl From<Duration> for SignedDuration {
    fn from(d: Duration) -> SignedDuration {
        SignedDuration {
            duration: d,
            is_positive: true
        }
    }
}

impl Ord for SignedDuration {
    fn cmp(&self, other: &SignedDuration) -> Ordering {
        self.as_nanos().cmp(&other.as_nanos())
    }
}

impl PartialOrd for SignedDuration {
    fn partial_cmp(&self, other: &SignedDuration) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Sub for SignedDuration {
    type Output = SignedDuration;

    fn sub(self, rhs: SignedDuration) -> SignedDuration {
        SignedDuration::from_nanos(self.as_nanos() - rhs.as_nanos())
    }
}

impl fmt::Debug for SignedDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_positive {
            write!(f, "+")?;
        } else {
            write!(f, "-")?;
        }

        write!(f, "{:?}", self.duration)
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use super::SignedDuration;

    #[test]
    fn op_subtract() {
        let zero_d = Duration::from_nanos(0);
        let one_d = Duration::from_nanos(1);
        let two_d = Duration::from_nanos(2);

        let zero_sd = SignedDuration::from(zero_d);
        let one_sd = SignedDuration::from(one_d);
        let neg_one_sd = SignedDuration { duration: one_d, is_positive: false };
        let two_sd = SignedDuration::from(two_d);
        let neg_two_sd = SignedDuration { duration: two_d, is_positive: false };

        assert_eq!(zero_d, zero_sd.duration);
        assert_eq!(true, zero_sd.is_positive);

        assert_eq!(zero_sd, zero_sd - zero_sd);

        assert_eq!(one_d, one_sd.duration);
        assert_eq!(true, one_sd.is_positive);

        assert_eq!(one_sd, one_sd - zero_sd);

        assert_eq!(one_d, neg_one_sd.duration);
        assert_eq!(false, neg_one_sd.is_positive);

        assert_eq!(neg_one_sd, neg_one_sd - zero_sd);

        assert_eq!(zero_sd, one_sd - one_sd);

        assert_eq!(one_sd, two_sd - one_sd);

        assert_eq!(neg_one_sd, one_sd - two_sd);

        assert_eq!(neg_two_sd, neg_one_sd - one_sd);

        assert_eq!(zero_sd, neg_one_sd - neg_one_sd);
    }
}
