use crate::*;

/// A pong response sent from the server
///
/// This should be a response to sending a PING to the server
#[derive(Clone, PartialEq)]
pub struct Pong<'a> {
    raw: Str<'a>,
    token: StrIndex,
}

impl<'a> Pong<'a> {
    raw!();
    str_field!(
        /// Token associated with the PONG event
        token
    );
}

impl<'a> FromIrcMessage<'a> for Pong<'a> {
    type Error = InvalidMessage;

    fn from_irc(msg: IrcMessage<'a>) -> Result<Self, Self::Error> {
        msg.expect_command(IrcMessage::PONG)?;

        let this = Self {
            token: msg.expect_data_index()?,
            raw: msg.raw,
        };

        Ok(this)
    }
}

into_owned!(Pong { raw, token });
impl_custom_debug!(Pong { raw, token });
serde_struct!(Pong { raw, token });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irc;

    #[test]
    #[cfg(feature = "serde")]
    fn pong_serde() {
        let input = "PONG :1234567890\r\n";
        crate::serde::round_trip_json::<Pong>(input);
    }

    #[test]
    fn pong() {
        let input = "PONG :1234567890\r\n";
        for msg in irc::parse(input).map(|s| s.unwrap()) {
            let msg = Pong::from_irc(msg).unwrap();
            assert_eq!(msg.token(), "1234567890");
        }
    }
}