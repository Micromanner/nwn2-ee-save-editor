use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SecurityLimits {
    pub max_file_size: usize,
    pub max_resource_count: usize,
    pub max_resource_size: usize,
    pub max_string_length: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_file_size: 500 * 1024 * 1024,     // 500MB for ERF/HAK files
            max_resource_count: 100_000,          // Maximum resources
            max_resource_size: 100 * 1024 * 1024, // 100MB per resource
            max_string_length: 1024,              // Maximum string length
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErfVersion {
    V10, // 16-character resource names
    V11, // 32-character resource names
}

impl ErfVersion {
    pub fn key_entry_size(&self) -> usize {
        match self {
            ErfVersion::V10 => 24,
            ErfVersion::V11 => 40,
        }
    }

    pub fn max_resource_name_length(&self) -> usize {
        match self {
            ErfVersion::V10 => 16,
            ErfVersion::V11 => 32,
        }
    }

    pub fn version_bytes(&self) -> &'static [u8; 4] {
        match self {
            ErfVersion::V10 => b"V1.0",
            ErfVersion::V11 => b"V1.1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErfType {
    ERF,
    HAK,
    MOD,
}

impl ErfType {
    pub fn from_signature(sig: &[u8; 4]) -> Option<Self> {
        match sig {
            b"ERF " => Some(ErfType::ERF),
            b"HAK " => Some(ErfType::HAK),
            b"MOD " => Some(ErfType::MOD),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ErfType::ERF => "ERF",
            ErfType::HAK => "HAK",
            ErfType::MOD => "MOD",
        }
    }

    pub fn signature(&self) -> &'static [u8; 4] {
        match self {
            ErfType::ERF => b"ERF ",
            ErfType::HAK => b"HAK ",
            ErfType::MOD => b"MOD ",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErfHeader {
    pub file_type: String,
    pub version: String,
    pub language_count: u32,
    pub localized_string_size: u32,
    pub entry_count: u32,
    pub offset_to_localized_string: u32,
    pub offset_to_key_list: u32,
    pub offset_to_resource_list: u32,
    pub build_year: u32,
    pub build_day: u32,
    pub description_str_ref: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    pub resource_name: String,
    pub resource_id: u32,
    pub resource_type: u16,
    pub reserved: u16,
}

impl KeyEntry {
    /// `resref.ext`; an unknown type keeps its numeric id as the extension so
    /// two unknown types never share a key.
    pub fn full_name(&self) -> String {
        match resource_type_to_extension(self.resource_type) {
            Some(ext) => format!("{}.{ext}", self.resource_name),
            None => format!("{}.{}", self.resource_name, self.resource_type),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEntry {
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone)]
pub struct ErfResource {
    pub key: KeyEntry,
    pub entry: ResourceEntry,
    pub data: Option<Vec<u8>>, // Lazy-loaded
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErfStatistics {
    pub total_resources: usize,
    pub total_size: usize,
    pub resource_types: HashMap<u16, usize>,
    pub largest_resource: Option<(String, usize)>,
    pub parse_time_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_path: String,
    pub file_size: usize,
    pub erf_type: String,
    pub version: String,
    pub build_date: String,
}

/// NWN2 resource type ids (xoreos list; ids observed in shipped modules,
/// HAKs and save `.z` archives match it).
pub fn resource_type_to_extension(resource_type: u16) -> Option<&'static str> {
    Some(match resource_type {
        0 => "res",
        1 => "bmp",
        2 => "mve",
        3 => "tga",
        4 => "wav",
        5 => "wfx",
        6 => "plt",
        7 => "ini",
        8 => "mp3",
        9 => "mpg",
        10 => "txt",
        2000 => "plh",
        2001 => "tex",
        2002 => "mdl",
        2003 => "thg",
        2005 => "fnt",
        2007 => "lua",
        2008 => "slt",
        2009 => "nss",
        2010 => "ncs",
        2011 => "mod",
        2012 => "are",
        2013 => "set",
        2014 => "ifo",
        2015 => "bic",
        2016 => "wok",
        2017 => "2da",
        2018 => "tlk",
        2022 => "txi",
        2023 => "git",
        2024 => "bti",
        2025 => "uti",
        2026 => "btc",
        2027 => "utc",
        2029 => "dlg",
        2030 => "itp",
        2031 => "btt",
        2032 => "utt",
        2033 => "dds",
        2034 => "bts",
        2035 => "uts",
        2036 => "ltr",
        2037 => "gff",
        2038 => "fac",
        2039 => "bte",
        2040 => "ute",
        2041 => "btd",
        2042 => "utd",
        2043 => "btp",
        2044 => "utp",
        2045 => "dft",
        2046 => "gic",
        2047 => "gui",
        2048 => "css",
        2049 => "ccs",
        2050 => "btm",
        2051 => "utm",
        2052 => "dwk",
        2053 => "pwk",
        2054 => "btg",
        2055 => "utg",
        2056 => "jrl",
        2057 => "sav",
        2058 => "utw",
        2059 => "4pc",
        2060 => "ssf",
        2061 => "hak",
        2062 => "nwm",
        2063 => "bik",
        2064 => "ndb",
        2065 => "ptm",
        2066 => "ptt",
        2067 => "bak",
        3000 => "osc",
        3001 => "usc",
        3002 => "trn",
        3003 => "utr",
        3004 => "uen",
        3005 => "ult",
        3006 => "sef",
        3007 => "pfx",
        3008 => "cam",
        3009 => "lfx",
        3010 => "bfx",
        3011 => "upe",
        3012 => "ros",
        3013 => "rst",
        3014 => "ifx",
        3015 => "pfb",
        3016 => "zip",
        3017 => "wmp",
        3018 => "bbx",
        3019 => "tfx",
        3020 => "wlk",
        3021 => "xml",
        3022 => "scc",
        3033 => "ptx",
        3034 => "ltx",
        3035 => "trx",
        4000 => "mdb",
        4001 => "mda",
        4002 => "spt",
        4003 => "gr2",
        4004 => "fxa",
        4005 => "fxe",
        4007 => "jpg",
        4008 => "pwc",
        _ => return None,
    })
}
