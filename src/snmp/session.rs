use crate::{error::AppError, snmp::credentials::SnmpCredentials};
use snmp2::{
    Oid, Value,
    v3::{Auth, AuthProtocol, Cipher, Security},
};
use std::{net::ToSocketAddrs, time::Duration};

pub trait ReadName {
    fn read_name<'a>(&self, buffer: &'a mut [u32]) -> Result<&'a [u32], snmp2::Error>;
}

impl ReadName for Oid<'_> {
    fn read_name<'a>(&self, buffer: &'a mut [u32]) -> Result<&'a [u32], snmp2::Error> {
        let values: Vec<_> = self.iter().ok_or(snmp2::Error::AsnParse)?.collect();
        if values.len() > buffer.len() {
            return Err(snmp2::Error::AsnParse);
        }
        for (index, value) in values.iter().enumerate() {
            buffer[index] = u32::try_from(*value).map_err(|_| snmp2::Error::AsnParse)?;
        }
        Ok(&buffer[..values.len()])
    }
}

pub struct Name<'a>(Oid<'a>);
impl ReadName for Name<'_> {
    fn read_name<'a>(&self, buffer: &'a mut [u32]) -> Result<&'a [u32], snmp2::Error> {
        self.0.read_name(buffer)
    }
}
impl PartialEq<&[u32]> for Name<'_> {
    fn eq(&self, other: &&[u32]) -> bool {
        let mut buffer = [0; 128];
        self.read_name(&mut buffer).is_ok_and(|name| name == *other)
    }
}

pub struct Response<'a> {
    pub error_status: u32,
    pub error_index: u32,
    pub varbinds: std::vec::IntoIter<(Name<'a>, Value<'a>)>,
}

pub struct SyncSession(snmp2::SyncSession);

impl SyncSession {
    pub fn new(
        destination: impl ToSocketAddrs,
        community: &[u8],
        timeout: Option<Duration>,
        request: i32,
    ) -> std::io::Result<Self> {
        snmp2::SyncSession::new_v2c(destination, community, timeout, request).map(Self)
    }

    pub fn authenticated(
        destination: impl ToSocketAddrs,
        credentials: &SnmpCredentials,
        timeout: Duration,
    ) -> Result<Self, AppError> {
        credentials.validate_resolved()?;
        let protocol = match credentials.v3_auth_protocol.as_deref() {
            Some("sha1") => AuthProtocol::Sha1,
            Some("sha512") => AuthProtocol::Sha512,
            _ => AuthProtocol::Sha256,
        };
        let auth = if credentials.v3_auth_protocol.as_deref() == Some("none") {
            Auth::NoAuthNoPriv
        } else if credentials.v3_priv_protocol.as_deref() == Some("none") {
            Auth::AuthNoPriv
        } else {
            Auth::AuthPriv {
                cipher: Cipher::Aes128,
                privacy_password: credentials
                    .v3_priv_password
                    .as_deref()
                    .unwrap_or_default()
                    .as_bytes()
                    .to_vec(),
            }
        };
        let security = Security::new(
            credentials
                .v3_username
                .as_deref()
                .unwrap_or_default()
                .as_bytes(),
            credentials
                .v3_auth_password
                .as_deref()
                .unwrap_or_default()
                .as_bytes(),
        )
        .with_auth_protocol(protocol)
        .with_auth(auth);
        let mut session = snmp2::SyncSession::new_v3(destination, Some(timeout), 1, security)
            .map_err(|_| AppError::Validation("SNMPv3 session initialization failed".into()))?;
        session.init().map_err(|_| {
            AppError::Validation("SNMPv3 engine discovery or authentication failed".into())
        })?;
        Ok(Self(session))
    }

    pub fn get(&mut self, oid: &[u32]) -> Result<Response<'_>, snmp2::Error> {
        let components: Vec<u64> = oid.iter().map(|value| u64::from(*value)).collect();
        let oid = Oid::from(&components).map_err(|_| snmp2::Error::AsnParse)?;
        Self::response(self.0.get(&oid)?)
    }

    pub fn getnext(&mut self, oid: &[u32]) -> Result<Response<'_>, snmp2::Error> {
        let components: Vec<u64> = oid.iter().map(|value| u64::from(*value)).collect();
        let oid = Oid::from(&components).map_err(|_| snmp2::Error::AsnParse)?;
        Self::response(self.0.getnext(&oid)?)
    }

    fn response(response: snmp2::Pdu<'_>) -> Result<Response<'_>, snmp2::Error> {
        Ok(Response {
            error_status: response.error_status,
            error_index: response.error_index,
            varbinds: response
                .varbinds
                .map(|(name, value)| {
                    let value = match value {
                        Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance => {
                            Value::Null
                        }
                        value => value,
                    };
                    (Name(name), value)
                })
                .collect::<Vec<_>>()
                .into_iter(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;

    fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut encoded = if value.len() < 128 {
            vec![tag, value.len() as u8]
        } else {
            vec![tag, 0x81, u8::try_from(value.len()).unwrap()]
        };
        encoded.extend_from_slice(value);
        encoded
    }

    #[test]
    fn v3_sessions_discover_engine_and_exchange_authenticated_encrypted_values() {
        for (protocol, algorithm, privacy) in [
            ("sha1", AuthProtocol::Sha1, "aes128"),
            ("sha256", AuthProtocol::Sha256, "aes128"),
            ("sha512", AuthProtocol::Sha512, "aes128"),
            ("sha256", AuthProtocol::Sha256, "none"),
            ("none", AuthProtocol::Sha256, "none"),
        ] {
            let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let address = socket.local_addr().unwrap();
            let worker = std::thread::spawn(move || {
                let mut buffer = [0; 4096];
                let (_, peer) = socket.recv_from(&mut buffer).unwrap();
                let engine = [0x80, 0, 0, 1, 1];
                let header = tlv(
                    0x30,
                    &[
                        tlv(2, &[1]),
                        tlv(2, &[0x7f, 0xff]),
                        tlv(4, &[0]),
                        tlv(2, &[3]),
                    ]
                    .concat(),
                );
                let security_parameters = tlv(
                    4,
                    &tlv(
                        0x30,
                        &[
                            tlv(4, &engine),
                            tlv(2, &[1]),
                            tlv(2, &[10]),
                            tlv(4, b""),
                            tlv(4, b""),
                            tlv(4, b""),
                        ]
                        .concat(),
                    ),
                );
                let report = tlv(0x30, &[tlv(2, &[3]), header, security_parameters].concat());
                socket.send_to(&report, peer).unwrap();
                let security = Security::new(b"test-user", b"test-auth-secret")
                    .with_auth_protocol(algorithm)
                    .with_auth(if protocol == "none" {
                        Auth::NoAuthNoPriv
                    } else if privacy == "none" {
                        Auth::AuthNoPriv
                    } else {
                        Auth::AuthPriv {
                            cipher: Cipher::Aes128,
                            privacy_password: b"test-privacy-secret".to_vec(),
                        }
                    })
                    .with_engine_id(&engine)
                    .unwrap()
                    .with_engine_boots_and_time(1, 10);
                let mut receive_security = security.clone();
                let (size, peer) = socket.recv_from(&mut buffer).unwrap();
                let mut request = snmp2::Pdu::from_bytes_with_security(
                    &buffer[..size],
                    Some(&mut receive_security),
                )
                .unwrap();
                assert_eq!(request.message_type, snmp2::MessageType::GetRequest);
                let bindings = tlv(
                    0x30,
                    &[
                        tlv(6, &[43, 6, 1, 2, 1, 1, 5, 0]),
                        tlv(4, b"v3-loopback-switch"),
                    ]
                    .concat(),
                );
                request.message_type = snmp2::MessageType::Response;
                request.varbinds = snmp2::Varbinds::from_bytes(&bindings);
                let response = request.to_bytes_with_security(Some(&security)).unwrap();
                assert_eq!(
                    response
                        .windows(b"v3-loopback-switch".len())
                        .any(|window| window == b"v3-loopback-switch"),
                    privacy == "none"
                );
                socket.send_to(&response, peer).unwrap();
            });
            let credentials = SnmpCredentials {
                version: Some("3".into()),
                v3_username: Some("test-user".into()),
                v3_auth_protocol: Some(protocol.into()),
                v3_auth_password: Some("test-auth-secret".into()),
                v3_priv_protocol: Some(privacy.into()),
                v3_priv_password: Some("test-privacy-secret".into()),
                ..Default::default()
            };
            let mut session =
                SyncSession::authenticated(address, &credentials, Duration::from_secs(3)).unwrap();
            let mut response = session.get(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap();
            assert!(matches!(
                response.varbinds.next().unwrap().1,
                Value::OctetString(b"v3-loopback-switch")
            ));
            worker.join().unwrap();
        }
    }
}
