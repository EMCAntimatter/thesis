use crate::message::client_message::ClientLogMessage;

impl<LogKeyType, LogValueType> ClientLogMessage<LogKeyType, LogValueType> {
    #[inline]
    pub fn tag(&self) -> u128 {
        // unsafe version
        *unsafe { std::mem::transmute::<&Self, &u128>(self) }

        // // safe version
        // ((self.timestamp as u128) << 64) | ((self.client_id.0 as u128) << 32) | (self.message_id.0 as u128)
    }
}

impl<LogKeyType, LogValueType> PartialEq for ClientLogMessage<LogKeyType, LogValueType> {
    fn eq(&self, other: &Self) -> bool {
        self.tag() == other.tag()
    }
}

impl<LogKeyType, LogValueType> Eq for ClientLogMessage<LogKeyType, LogValueType> {}

impl<LogKeyType, LogValueType> PartialOrd for ClientLogMessage<LogKeyType, LogValueType> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<LogKeyType, LogValueType> Ord for ClientLogMessage<LogKeyType, LogValueType> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tag().cmp(&other.tag())
    }
}
