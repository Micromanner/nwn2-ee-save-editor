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
            max_file_size: 500 * 1024 * 1024,  // 500MB for ERF/HAK files
            max_resource_count: 100_000,        // Maximum resources
            max_resource_size: 100 * 1024 * 1024, // 100MB per resource
            max_string_length: 1024,            // Maximum string length
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErfVersion {
    V10,  // 16-character resource names
    V11,  // 32-character resource names
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
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.resource_name, resource_type_to_extension(self.resource_type))
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
    pub data: Option<Vec<u8>>,  // Lazy-loaded
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

pub fn resource_type_to_extension(resource_type: u16) -> &'static str {
    match resource_type {
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
        2068 => "osc",
        2069 => "usc",
        2070 => "trn",
        2071 => "utr",
        2072 => "uen",
        2073 => "ult",
        2074 => "sef",
        2075 => "pfx",
        2076 => "cam",
        2077 => "lfx",
        2078 => "bfx",
        2079 => "upe",
        2080 => "ros",
        2081 => "rst",
        2082 => "ifx",
        2083 => "pfb",
        2084 => "zip",
        2085 => "wmp",
        2086 => "bbx",
        2087 => "tfx",
        2088 => "wlk",
        2089 => "xml",
        2090 => "scc",
        2091 => "ptx",
        2092 => "ltx",
        2093 => "trx",
        3000 => "trn",
        3001 => "trx",
        3002 => "trn",
        3003 => "trx",
        3004 => "xml",
        3005 => "mdb",
        3006 => "mda",
        3007 => "spt",
        3008 => "gr2",
        3009 => "fxa",
        3010 => "fxe",
        3011 => "jpg",
        3012 => "pwc",
        3013 => "nwn2",
        3014 => "amc",
        3015 => "icc",
        3016 => "ogg",
        3017 => "con",
        3018 => "obr",
        3019 => "obs",
        3020 => "wdb",
        3021 => "stn",
        3022 => "lod",
        3023 => "wrw",
        3024 => "pfr",
        3025 => "emt",
        3026 => "gdc",
        3027 => "gdf",
        3028 => "gft",
        3029 => "crf",
        3030 => "cre",
        3031 => "crm",
        3032 => "crt",
        3033 => "wda",
        _ => "unk",
    }
}

pub fn extension_to_resource_type(ext: &str) -> Option<u16> {
    let ext_lower = ext.to_lowercase();
    match ext_lower.as_str() {
        "2da" => Some(2017),
        "tlk" => Some(2018),
        "gff" => Some(2037),
        "ifo" => Some(2014),
        "bic" => Some(2015),
        "uti" => Some(2025),
        "utc" => Some(2027),
        "utm" => Some(2051),
        "utp" => Some(2044),
        "utw" => Some(2058),
        "are" => Some(2012),
        "git" => Some(2023),
        "dlg" => Some(2029),
        "jrl" => Some(2056),
        "ros" => Some(2080),
        "rst" => Some(2081),
        "xml" => Some(2089),
        "nss" => Some(2009),
        "ncs" => Some(2010),
        "mod" => Some(2011),
        "hak" => Some(2061),
        "sav" => Some(2057),
        _ => None,
    }
}

pub struct ErfBuilder {
    erf_type: ErfType,
    version: ErfVersion,
    resources: Vec<(String, u16, Vec<u8>)>,
    build_year: u32,
    build_day: u32,
    description_str_ref: u32,
}

impl ErfBuilder {
    pub fn new(erf_type: ErfType) -> Self {
        Self {
            erf_type,
            version: ErfVersion::V11,
            resources: Vec::new(),
            build_year: 125,
            build_day: 1,
            description_str_ref: 0xFFFFFFFF,
        }
    }

    pub fn version(mut self, version: ErfVersion) -> Self {
        self.version = version;
        self
    }

    pub fn build_date(mut self, year: u32, day: u32) -> Self {
        self.build_year = year;
        self.build_day = day;
        self
    }

    pub fn description_str_ref(mut self, str_ref: u32) -> Self {
        self.description_str_ref = str_ref;
        self
    }

    pub fn add_resource(mut self, name: &str, data: Vec<u8>) -> Self {
        let resource_type = if let Some(dot_pos) = name.rfind('.') {
            let ext = &name[dot_pos + 1..];
            extension_to_resource_type(ext).unwrap_or(2037)
        } else {
            2037
        };
        self.resources.push((name.to_string(), resource_type, data));
        self
    }

    pub fn add_resource_with_type(mut self, name: &str, resource_type: u16, data: Vec<u8>) -> Self {
        self.resources.push((name.to_string(), resource_type, data));
        self
    }

    pub fn build(self) -> super::parser::ErfParser {
        use super::parser::ErfParser;

        let mut parser = ErfParser::new_archive(self.erf_type, self.version);

        if let Some(header) = &mut parser.header {
            header.build_year = self.build_year;
            header.build_day = self.build_day;
            header.description_str_ref = self.description_str_ref;
        }

        for (name, resource_type, data) in self.resources {
            let _ = parser.add_resource(&name, resource_type, data);
        }

        parser
    }
}