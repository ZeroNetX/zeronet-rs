use zeronet_protocol::ZeroConnection;

pub mod api;
pub mod builders;

pub struct Protocol<'a>(pub(crate) &'a mut ZeroConnection);

impl<'a> Protocol<'a> {
    pub fn new(connection: &'a mut ZeroConnection) -> Self {
        Protocol(connection)
    }
}
