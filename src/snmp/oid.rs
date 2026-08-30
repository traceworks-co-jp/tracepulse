#[derive(Debug, Clone, Copy)]
pub struct SnmpOid {
    pub value: &'static str,
}

impl SnmpOid {
    pub const SYS_NAME: Self = Self { value: "1.3.6.1.2.1.1.5.0" };
    pub const SYS_DESCR: Self = Self { value: "1.3.6.1.2.1.1.1.0" };
    pub const IF_NUMBER: Self = Self { value: "1.3.6.1.2.1.2.1.0" };
    pub const IF_OPER_STATUS: Self = Self { value: "1.3.6.1.2.1.2.2.1.8" };
    pub const IF_IN_ERRORS: Self = Self { value: "1.3.6.1.2.1.2.2.1.14" };
    pub const IF_OUT_ERRORS: Self = Self { value: "1.3.6.1.2.1.2.2.1.20" };
    pub const IF_IN_OCTETS: Self = Self { value: "1.3.6.1.2.1.2.2.1.10" };
    pub const IF_OUT_OCTETS: Self = Self { value: "1.3.6.1.2.1.2.2.1.16" };
    pub const CPU_LOAD: Self = Self { value: "1.3.6.1.4.1.2021.10.1.5.1" };
    pub const MEMORY_TOTAL: Self = Self { value: "1.3.6.1.4.1.2021.4.5.1.1" };
    pub const MEMORY_FREE: Self = Self { value: "1.3.6.1.4.1.2021.4.6.0" };
}
