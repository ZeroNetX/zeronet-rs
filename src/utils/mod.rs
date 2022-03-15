use std::default::Default;

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

pub fn gen_peer_id() -> String {
    let vec: Vec<u8> = (0..12).map(|_| rand::random::<u8>()).collect();
    let peer_id = format!("-UT3530-{}", base64::encode(&vec));
    peer_id
}
