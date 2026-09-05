use snmp::Value;

/// SNMP の値をオーナーシップ付きで保持する表現。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnmpValue {
    Boolean(bool),
    Integer(i64),
    Unsigned32(u32),
    Counter32(u32),
    Counter64(u64),
    Timeticks(u32),
    OctetString(Vec<u8>),
    Opaque(Vec<u8>),
    ObjectIdentifier(Vec<u32>),
    IpAddress([u8; 4]),
    Null,
    /// 上記以外（構造型など）。デバッグ用の表現のみを保持する。
    Unsupported(String),
}

impl SnmpValue {
    pub(crate) fn from_raw(value: &Value<'_>) -> Self {
        match value {
            Value::Boolean(v) => Self::Boolean(*v),
            Value::Integer(v) => Self::Integer(*v),
            Value::Unsigned32(v) => Self::Unsigned32(*v),
            Value::Counter32(v) => Self::Counter32(*v),
            Value::Counter64(v) => Self::Counter64(*v),
            Value::Timeticks(v) => Self::Timeticks(*v),
            Value::OctetString(bytes) => Self::OctetString(bytes.to_vec()),
            Value::Opaque(bytes) => Self::Opaque(bytes.to_vec()),
            Value::IpAddress(addr) => Self::IpAddress(*addr),
            Value::Null => Self::Null,
            Value::ObjectIdentifier(oid) => {
                let mut buf = [0u32; 128];
                match oid.read_name(&mut buf) {
                    Ok(parsed) => Self::ObjectIdentifier(parsed.to_vec()),
                    Err(_) => Self::Unsupported("OBJECT IDENTIFIER (unreadable)".to_string()),
                }
            }
            other => Self::Unsupported(format!("{:?}", other)),
        }
    }

    /// 文字列として解釈できる値を返す（OctetString は UTF-8 lossy）。
    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::OctetString(bytes) | Self::Opaque(bytes) => {
                Some(String::from_utf8_lossy(bytes).to_string())
            }
            Self::ObjectIdentifier(oid) => Some(oid_to_dotted(oid)),
            Self::Integer(v) => Some(v.to_string()),
            Self::Unsigned32(v) | Self::Counter32(v) | Self::Timeticks(v) => Some(v.to_string()),
            Self::Counter64(v) => Some(v.to_string()),
            Self::Boolean(v) => Some(v.to_string()),
            Self::IpAddress(addr) => {
                Some(format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3]))
            }
            Self::Null | Self::Unsupported(_) => None,
        }
    }

    /// 数値として解釈できる値を返す。
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Integer(v) => u64::try_from(*v).ok(),
            Self::Unsigned32(v) | Self::Counter32(v) | Self::Timeticks(v) => Some(u64::from(*v)),
            Self::Counter64(v) => Some(*v),
            _ => None,
        }
    }

    /// バイト列（LLDP の ChassisId / PortId など）を返す。
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::OctetString(bytes) | Self::Opaque(bytes) => Some(bytes.as_slice()),
            _ => None,
        }
    }
}

/// Walk で取得した OID と値のペア。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnmpVarBind {
    pub oid: Vec<u32>,
    pub value: SnmpValue,
}

impl SnmpVarBind {
    pub fn oid_string(&self) -> String {
        oid_to_dotted(&self.oid)
    }

    /// 指定プレフィックス配下のインデックス部（テーブル行キー）を返す。
    pub fn index_suffix(&self, prefix: &[u32]) -> Option<&[u32]> {
        self.oid.strip_prefix(prefix)
    }
}

fn oid_to_dotted(oid: &[u32]) -> String {
    oid.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varbind_exposes_oid_and_index_suffix() {
        // lldpRemSysName.0.7.1
        let prefix = [1u32, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 9];
        let varbind = SnmpVarBind {
            oid: [prefix.as_slice(), &[0, 7, 1]].concat(),
            value: SnmpValue::OctetString(b"core-sw-01".to_vec()),
        };

        assert_eq!(varbind.oid_string(), "1.0.8802.1.1.2.1.4.1.1.9.0.7.1");
        assert_eq!(varbind.index_suffix(&prefix), Some([0u32, 7, 1].as_slice()));
        assert_eq!(varbind.value.as_string().as_deref(), Some("core-sw-01"));
    }

    #[test]
    fn index_suffix_returns_none_for_other_prefix() {
        let varbind = SnmpVarBind {
            oid: vec![1, 3, 6, 1, 2, 1, 1, 5, 0],
            value: SnmpValue::Null,
        };
        assert_eq!(varbind.index_suffix(&[1, 0, 8802]), None);
    }
}
