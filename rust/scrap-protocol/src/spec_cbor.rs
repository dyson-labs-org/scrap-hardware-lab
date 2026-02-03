use crate::ProtocolError;
use minicbor::data::Type;
use minicbor::{Decoder, Encoder};

impl From<minicbor::decode::Error> for ProtocolError {
    fn from(err: minicbor::decode::Error) -> Self {
        ProtocolError::new(err.to_string())
    }
}

impl<E: core::fmt::Display> From<minicbor::encode::Error<E>> for ProtocolError {
    fn from(err: minicbor::encode::Error<E>) -> Self {
        ProtocolError::new(err.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapHeader {
    pub alg: String,
    pub typ: String,
    pub enc: Option<String>,
    pub chn: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeoBounds {
    pub lat_min: Option<f64>,
    pub lat_max: Option<f64>,
    pub lon_min: Option<f64>,
    pub lon_max: Option<f64>,
    pub polygon: Option<Vec<[f64; 2]>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeWindow {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Constraints {
    pub max_area_km2: Option<u64>,
    pub max_range_km: Option<f64>,
    pub max_hops: Option<u32>,
    pub geographic_bounds: Option<GeoBounds>,
    pub time_window: Option<TimeWindow>,
    pub min_approach_distance_m: Option<u64>,
    pub max_relative_velocity_m_s: Option<f64>,
    pub fuel_budget_kg: Option<f64>,
    pub abort_triggers: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapPayload {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub iat: u64,
    pub exp: u64,
    pub jti: String,
    pub cap: Vec<String>,
    pub cns: Option<Constraints>,
    pub prf: Option<String>,
    pub cmd_pub: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SatCapToken {
    pub header: CapHeader,
    pub payload: CapPayload,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundTaskRequest {
    pub capability_token: Vec<u8>,
    pub payment_hash: Vec<u8>,
    pub payment_amount_msat: u64,
    pub htlc_timeout_blocks: u32,
    pub binding_sig: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputMetadata {
    pub data_size_bytes: Option<u64>,
    pub data_format: Option<String>,
    pub coverage_km2: Option<f64>,
    pub acquisition_start: Option<u64>,
    pub acquisition_end: Option<u64>,
    pub sensor_mode: Option<String>,
    pub content_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub storage_location: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionProof {
    pub task_jti: String,
    pub payment_hash: Vec<u8>,
    pub output_hash: Vec<u8>,
    pub execution_timestamp: u64,
    pub output_metadata: Option<OutputMetadata>,
    pub executor_sig: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAccepted {
    pub task_jti: String,
    pub accepted_at: u64,
    pub estimated_completion: u64,
    pub executor_sig: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRejected {
    pub task_jti: String,
    pub rejected_at: u64,
    pub reason: String,
    pub detail: Option<String>,
    pub executor_sig: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLocation {
    pub method: String,
    pub relay_satellite: Option<String>,
    pub ground_station: Option<String>,
    pub estimated_delivery: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskCompleted {
    pub task_jti: String,
    pub proof: ExecutionProof,
    pub data_location: Option<DataLocation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskFailed {
    pub task_jti: String,
    pub failed_at: u64,
    pub reason: String,
    pub detail: Option<String>,
    pub partial_proof: Option<ExecutionProof>,
    pub executor_sig: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskResponse {
    Accepted(TaskAccepted),
    Rejected(TaskRejected),
    Completed(TaskCompleted),
    Failed(TaskFailed),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    TaskRequest,
    TaskResponse,
    Proof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScapPayload {
    TaskRequest(BoundTaskRequest),
    TaskResponse(TaskResponse),
    Proof(ExecutionProof),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IslScapMessage {
    pub version: u64,
    pub msg_type: MessageType,
    pub sender: String,
    pub recipient: String,
    pub sequence: u64,
    pub timestamp: u64,
    pub payload: ScapPayload,
    pub hmac: Option<Vec<u8>>,
}

impl CapHeader {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 2;
        if self.enc.is_some() {
            len += 1;
        }
        if self.chn.is_some() {
            len += 1;
        }
        enc.map(len)?;
        enc.str("alg")?;
        enc.str(&self.alg)?;
        enc.str("typ")?;
        enc.str(&self.typ)?;
        if let Some(enc_val) = &self.enc {
            enc.str("enc")?;
            enc.str(enc_val)?;
        }
        if let Some(chn) = self.chn {
            enc.str("chn")?;
            enc.u32(chn)?;
        }
        Ok(())
    }

    pub fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        self.encode_into(&mut enc)?;
        Ok(buf)
    }

    pub fn decode_cbor(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut dec = Decoder::new(bytes);
        Self::decode_from(&mut dec)
    }

    fn decode_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut alg: Option<String> = None;
        let mut typ: Option<String> = None;
        let mut enc: Option<String> = None;
        let mut chn: Option<u32> = None;

        decode_map(dec, |key, dec| {
            match key {
                "alg" => alg = Some(dec.str()?.to_string()),
                "typ" => typ = Some(dec.str()?.to_string()),
                "enc" => enc = Some(dec.str()?.to_string()),
                "chn" => chn = Some(decode_u32(dec)?),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(CapHeader {
            alg: alg.ok_or_else(|| ProtocolError::new("cap header missing alg"))?,
            typ: typ.ok_or_else(|| ProtocolError::new("cap header missing typ"))?,
            enc,
            chn,
        })
    }
}

impl GeoBounds {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 0;
        if self.lat_min.is_some() {
            len += 1;
        }
        if self.lat_max.is_some() {
            len += 1;
        }
        if self.lon_min.is_some() {
            len += 1;
        }
        if self.lon_max.is_some() {
            len += 1;
        }
        if self.polygon.is_some() {
            len += 1;
        }
        enc.map(len)?;
        if let Some(val) = self.lat_min {
            enc.str("lat_min")?;
            enc.f64(val)?;
        }
        if let Some(val) = self.lat_max {
            enc.str("lat_max")?;
            enc.f64(val)?;
        }
        if let Some(val) = self.lon_min {
            enc.str("lon_min")?;
            enc.f64(val)?;
        }
        if let Some(val) = self.lon_max {
            enc.str("lon_max")?;
            enc.f64(val)?;
        }
        if let Some(poly) = &self.polygon {
            enc.str("polygon")?;
            enc.array(poly.len() as u64)?;
            for point in poly {
                enc.array(2)?;
                enc.f64(point[0])?;
                enc.f64(point[1])?;
            }
        }
        Ok(())
    }

    fn decode_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut lat_min = None;
        let mut lat_max = None;
        let mut lon_min = None;
        let mut lon_max = None;
        let mut polygon: Option<Vec<[f64; 2]>> = None;

        decode_map(dec, |key, dec| {
            match key {
                "lat_min" => lat_min = Some(decode_f64(dec)?),
                "lat_max" => lat_max = Some(decode_f64(dec)?),
                "lon_min" => lon_min = Some(decode_f64(dec)?),
                "lon_max" => lon_max = Some(decode_f64(dec)?),
                "polygon" => {
                    let len = dec.array()?.unwrap_or(0);
                    let mut points = Vec::new();
                    for _ in 0..len {
                        let _ = dec.array()?;
                        let lat = decode_f64(dec)?;
                        let lon = decode_f64(dec)?;
                        points.push([lat, lon]);
                    }
                    polygon = Some(points);
                }
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(GeoBounds {
            lat_min,
            lat_max,
            lon_min,
            lon_max,
            polygon,
        })
    }
}

impl TimeWindow {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        enc.map(2)?;
        enc.str("start")?;
        enc.u64(self.start)?;
        enc.str("end")?;
        enc.u64(self.end)?;
        Ok(())
    }

    fn decode_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut start = None;
        let mut end = None;
        decode_map(dec, |key, dec| {
            match key {
                "start" => start = Some(dec.u64()?),
                "end" => end = Some(dec.u64()?),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;
        Ok(TimeWindow {
            start: start.ok_or_else(|| ProtocolError::new("time_window missing start"))?,
            end: end.ok_or_else(|| ProtocolError::new("time_window missing end"))?,
        })
    }
}

impl Constraints {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 0;
        if self.max_area_km2.is_some() {
            len += 1;
        }
        if self.max_range_km.is_some() {
            len += 1;
        }
        if self.max_hops.is_some() {
            len += 1;
        }
        if self.geographic_bounds.is_some() {
            len += 1;
        }
        if self.time_window.is_some() {
            len += 1;
        }
        if self.min_approach_distance_m.is_some() {
            len += 1;
        }
        if self.max_relative_velocity_m_s.is_some() {
            len += 1;
        }
        if self.fuel_budget_kg.is_some() {
            len += 1;
        }
        if self.abort_triggers.is_some() {
            len += 1;
        }
        enc.map(len)?;
        if let Some(val) = self.max_area_km2 {
            enc.str("max_area_km2")?;
            enc.u64(val)?;
        }
        if let Some(val) = self.max_range_km {
            enc.str("max_range_km")?;
            enc.f64(val)?;
        }
        if let Some(val) = self.max_hops {
            enc.str("max_hops")?;
            enc.u32(val)?;
        }
        if let Some(bounds) = &self.geographic_bounds {
            enc.str("geographic_bounds")?;
            bounds.encode_into(enc)?;
        }
        if let Some(window) = &self.time_window {
            enc.str("time_window")?;
            window.encode_into(enc)?;
        }
        if let Some(val) = self.min_approach_distance_m {
            enc.str("min_approach_distance_m")?;
            enc.u64(val)?;
        }
        if let Some(val) = self.max_relative_velocity_m_s {
            enc.str("max_relative_velocity_m_s")?;
            enc.f64(val)?;
        }
        if let Some(val) = self.fuel_budget_kg {
            enc.str("fuel_budget_kg")?;
            enc.f64(val)?;
        }
        if let Some(triggers) = &self.abort_triggers {
            enc.str("abort_triggers")?;
            enc.array(triggers.len() as u64)?;
            for trigger in triggers {
                enc.str(trigger)?;
            }
        }
        Ok(())
    }

    fn decode_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut max_area_km2 = None;
        let mut max_range_km = None;
        let mut max_hops = None;
        let mut geographic_bounds = None;
        let mut time_window = None;
        let mut min_approach_distance_m = None;
        let mut max_relative_velocity_m_s = None;
        let mut fuel_budget_kg = None;
        let mut abort_triggers = None;

        decode_map(dec, |key, dec| {
            match key {
                "max_area_km2" => max_area_km2 = Some(dec.u64()?),
                "max_range_km" => max_range_km = Some(decode_f64(dec)?),
                "max_hops" => max_hops = Some(decode_u32(dec)?),
                "geographic_bounds" => geographic_bounds = Some(GeoBounds::decode_from(dec)?),
                "time_window" => time_window = Some(TimeWindow::decode_from(dec)?),
                "min_approach_distance_m" => min_approach_distance_m = Some(dec.u64()?),
                "max_relative_velocity_m_s" => max_relative_velocity_m_s = Some(decode_f64(dec)?),
                "fuel_budget_kg" => fuel_budget_kg = Some(decode_f64(dec)?),
                "abort_triggers" => {
                    let len = dec.array()?.unwrap_or(0);
                    let mut items = Vec::new();
                    for _ in 0..len {
                        items.push(dec.str()?.to_string());
                    }
                    abort_triggers = Some(items);
                }
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(Constraints {
            max_area_km2,
            max_range_km,
            max_hops,
            geographic_bounds,
            time_window,
            min_approach_distance_m,
            max_relative_velocity_m_s,
            fuel_budget_kg,
            abort_triggers,
        })
    }
}

impl CapPayload {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 7;
        if self.cns.is_some() {
            len += 1;
        }
        if self.prf.is_some() {
            len += 1;
        }
        if self.cmd_pub.is_some() {
            len += 1;
        }
        enc.map(len)?;
        enc.str("iss")?;
        enc.str(&self.iss)?;
        enc.str("sub")?;
        enc.str(&self.sub)?;
        enc.str("aud")?;
        enc.str(&self.aud)?;
        enc.str("iat")?;
        enc.u64(self.iat)?;
        enc.str("exp")?;
        enc.u64(self.exp)?;
        enc.str("jti")?;
        enc.str(&self.jti)?;
        enc.str("cap")?;
        enc.array(self.cap.len() as u64)?;
        for cap in &self.cap {
            enc.str(cap)?;
        }
        if let Some(constraints) = &self.cns {
            enc.str("cns")?;
            constraints.encode_into(enc)?;
        }
        if let Some(prf) = &self.prf {
            enc.str("prf")?;
            enc.str(prf)?;
        }
        if let Some(cmd_pub) = &self.cmd_pub {
            enc.str("cmd_pub")?;
            enc.bytes(cmd_pub)?;
        }
        Ok(())
    }

    fn decode_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut iss = None;
        let mut sub = None;
        let mut aud = None;
        let mut iat = None;
        let mut exp = None;
        let mut jti = None;
        let mut cap = None;
        let mut cns = None;
        let mut prf = None;
        let mut cmd_pub = None;

        decode_map(dec, |key, dec| {
            match key {
                "iss" => iss = Some(dec.str()?.to_string()),
                "sub" => sub = Some(dec.str()?.to_string()),
                "aud" => aud = Some(dec.str()?.to_string()),
                "iat" => iat = Some(dec.u64()?),
                "exp" => exp = Some(dec.u64()?),
                "jti" => jti = Some(dec.str()?.to_string()),
                "cap" => {
                    let len = dec.array()?.unwrap_or(0);
                    let mut caps = Vec::new();
                    for _ in 0..len {
                        caps.push(dec.str()?.to_string());
                    }
                    cap = Some(caps);
                }
                "cns" => cns = Some(Constraints::decode_from(dec)?),
                "prf" => prf = Some(dec.str()?.to_string()),
                "cmd_pub" => cmd_pub = Some(dec.bytes()?.to_vec()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(CapPayload {
            iss: iss.ok_or_else(|| ProtocolError::new("cap payload missing iss"))?,
            sub: sub.ok_or_else(|| ProtocolError::new("cap payload missing sub"))?,
            aud: aud.ok_or_else(|| ProtocolError::new("cap payload missing aud"))?,
            iat: iat.ok_or_else(|| ProtocolError::new("cap payload missing iat"))?,
            exp: exp.ok_or_else(|| ProtocolError::new("cap payload missing exp"))?,
            jti: jti.ok_or_else(|| ProtocolError::new("cap payload missing jti"))?,
            cap: cap.ok_or_else(|| ProtocolError::new("cap payload missing cap"))?,
            cns,
            prf,
            cmd_pub,
        })
    }
}

impl SatCapToken {
    pub fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        enc.map(3)?;
        enc.str("header")?;
        self.header.encode_into(&mut enc)?;
        enc.str("payload")?;
        self.payload.encode_into(&mut enc)?;
        enc.str("signature")?;
        enc.bytes(&self.signature)?;
        Ok(buf)
    }

    pub fn decode_cbor(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut dec = Decoder::new(bytes);
        let mut header: Option<CapHeader> = None;
        let mut payload: Option<CapPayload> = None;
        let mut signature: Option<Vec<u8>> = None;

        decode_map(&mut dec, |key, dec| {
            match key {
                "header" => header = Some(CapHeader::decode_from(dec)?),
                "payload" => payload = Some(CapPayload::decode_from(dec)?),
                "signature" => signature = Some(dec.bytes()?.to_vec()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(SatCapToken {
            header: header.ok_or_else(|| ProtocolError::new("token missing header"))?,
            payload: payload.ok_or_else(|| ProtocolError::new("token missing payload"))?,
            signature: signature.ok_or_else(|| ProtocolError::new("token missing signature"))?,
        })
    }
}

impl BoundTaskRequest {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        enc.map(5)?;
        enc.str("capability_token")?;
        enc.bytes(&self.capability_token)?;
        enc.str("payment_hash")?;
        enc.bytes(&self.payment_hash)?;
        enc.str("payment_amount_msat")?;
        enc.u64(self.payment_amount_msat)?;
        enc.str("htlc_timeout_blocks")?;
        enc.u32(self.htlc_timeout_blocks)?;
        enc.str("binding_sig")?;
        enc.bytes(&self.binding_sig)?;
        Ok(())
    }

    pub fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        self.encode_into(&mut enc)?;
        Ok(buf)
    }

    pub fn decode_cbor(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut dec = Decoder::new(bytes);
        let mut capability_token = None;
        let mut payment_hash = None;
        let mut payment_amount_msat = None;
        let mut htlc_timeout_blocks = None;
        let mut binding_sig = None;

        decode_map(&mut dec, |key, dec| {
            match key {
                "capability_token" => capability_token = Some(dec.bytes()?.to_vec()),
                "payment_hash" => payment_hash = Some(dec.bytes()?.to_vec()),
                "payment_amount_msat" => payment_amount_msat = Some(dec.u64()?),
                "htlc_timeout_blocks" => htlc_timeout_blocks = Some(decode_u32(dec)?),
                "binding_sig" => binding_sig = Some(dec.bytes()?.to_vec()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(BoundTaskRequest {
            capability_token: capability_token
                .ok_or_else(|| ProtocolError::new("bound task missing capability_token"))?,
            payment_hash: payment_hash
                .ok_or_else(|| ProtocolError::new("bound task missing payment_hash"))?,
            payment_amount_msat: payment_amount_msat
                .ok_or_else(|| ProtocolError::new("bound task missing payment_amount_msat"))?,
            htlc_timeout_blocks: htlc_timeout_blocks
                .ok_or_else(|| ProtocolError::new("bound task missing htlc_timeout_blocks"))?,
            binding_sig: binding_sig
                .ok_or_else(|| ProtocolError::new("bound task missing binding_sig"))?,
        })
    }
}

impl OutputMetadata {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 0;
        if self.data_size_bytes.is_some() {
            len += 1;
        }
        if self.data_format.is_some() {
            len += 1;
        }
        if self.coverage_km2.is_some() {
            len += 1;
        }
        if self.acquisition_start.is_some() {
            len += 1;
        }
        if self.acquisition_end.is_some() {
            len += 1;
        }
        if self.sensor_mode.is_some() {
            len += 1;
        }
        if self.content_type.is_some() {
            len += 1;
        }
        if self.size_bytes.is_some() {
            len += 1;
        }
        if self.storage_location.is_some() {
            len += 1;
        }
        enc.map(len)?;
        if let Some(val) = self.data_size_bytes {
            enc.str("data_size_bytes")?;
            enc.u64(val)?;
        }
        if let Some(val) = &self.data_format {
            enc.str("data_format")?;
            enc.str(val)?;
        }
        if let Some(val) = self.coverage_km2 {
            enc.str("coverage_km2")?;
            enc.f64(val)?;
        }
        if let Some(val) = self.acquisition_start {
            enc.str("acquisition_start")?;
            enc.u64(val)?;
        }
        if let Some(val) = self.acquisition_end {
            enc.str("acquisition_end")?;
            enc.u64(val)?;
        }
        if let Some(val) = &self.sensor_mode {
            enc.str("sensor_mode")?;
            enc.str(val)?;
        }
        if let Some(val) = &self.content_type {
            enc.str("content_type")?;
            enc.str(val)?;
        }
        if let Some(val) = self.size_bytes {
            enc.str("size_bytes")?;
            enc.u64(val)?;
        }
        if let Some(val) = &self.storage_location {
            enc.str("storage_location")?;
            enc.str(val)?;
        }
        Ok(())
    }

    fn decode_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut data_size_bytes = None;
        let mut data_format = None;
        let mut coverage_km2 = None;
        let mut acquisition_start = None;
        let mut acquisition_end = None;
        let mut sensor_mode = None;
        let mut content_type = None;
        let mut size_bytes = None;
        let mut storage_location = None;

        decode_map(dec, |key, dec| {
            match key {
                "data_size_bytes" => data_size_bytes = Some(dec.u64()?),
                "data_format" => data_format = Some(dec.str()?.to_string()),
                "coverage_km2" => coverage_km2 = Some(decode_f64(dec)?),
                "acquisition_start" => acquisition_start = Some(dec.u64()?),
                "acquisition_end" => acquisition_end = Some(dec.u64()?),
                "sensor_mode" => sensor_mode = Some(dec.str()?.to_string()),
                "content_type" => content_type = Some(dec.str()?.to_string()),
                "size_bytes" => size_bytes = Some(dec.u64()?),
                "storage_location" => storage_location = Some(dec.str()?.to_string()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(OutputMetadata {
            data_size_bytes,
            data_format,
            coverage_km2,
            acquisition_start,
            acquisition_end,
            sensor_mode,
            content_type,
            size_bytes,
            storage_location,
        })
    }
}

impl ExecutionProof {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 5;
        if self.output_metadata.is_some() {
            len += 1;
        }
        enc.map(len)?;
        enc.str("task_jti")?;
        enc.str(&self.task_jti)?;
        enc.str("payment_hash")?;
        enc.bytes(&self.payment_hash)?;
        enc.str("output_hash")?;
        enc.bytes(&self.output_hash)?;
        enc.str("execution_timestamp")?;
        enc.u64(self.execution_timestamp)?;
        if let Some(meta) = &self.output_metadata {
            enc.str("output_metadata")?;
            meta.encode_into(enc)?;
        }
        enc.str("executor_sig")?;
        enc.bytes(&self.executor_sig)?;
        Ok(())
    }

    pub fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        self.encode_into(&mut enc)?;
        Ok(buf)
    }

    pub fn decode_cbor(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut dec = Decoder::new(bytes);
        let mut task_jti = None;
        let mut payment_hash = None;
        let mut output_hash = None;
        let mut execution_timestamp = None;
        let mut output_metadata = None;
        let mut executor_sig = None;

        decode_map(&mut dec, |key, dec| {
            match key {
                "task_jti" => task_jti = Some(dec.str()?.to_string()),
                "payment_hash" => payment_hash = Some(dec.bytes()?.to_vec()),
                "output_hash" => output_hash = Some(dec.bytes()?.to_vec()),
                "execution_timestamp" => execution_timestamp = Some(dec.u64()?),
                "output_metadata" => output_metadata = Some(OutputMetadata::decode_from(dec)?),
                "executor_sig" => executor_sig = Some(dec.bytes()?.to_vec()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(ExecutionProof {
            task_jti: task_jti.ok_or_else(|| ProtocolError::new("proof missing task_jti"))?,
            payment_hash: payment_hash
                .ok_or_else(|| ProtocolError::new("proof missing payment_hash"))?,
            output_hash: output_hash
                .ok_or_else(|| ProtocolError::new("proof missing output_hash"))?,
            execution_timestamp: execution_timestamp
                .ok_or_else(|| ProtocolError::new("proof missing execution_timestamp"))?,
            output_metadata,
            executor_sig: executor_sig
                .ok_or_else(|| ProtocolError::new("proof missing executor_sig"))?,
        })
    }
}

impl TaskResponse {
    pub fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        match self {
            TaskResponse::Accepted(accepted) => accepted.encode_cbor(),
            TaskResponse::Rejected(rejected) => rejected.encode_cbor(),
            TaskResponse::Completed(completed) => completed.encode_cbor(),
            TaskResponse::Failed(failed) => failed.encode_cbor(),
        }
    }

    pub fn decode_cbor(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut dec = Decoder::new(bytes);
        Self::decode_from(&mut dec)
    }

    fn decode_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut variant: Option<String> = None;
        let mut task_jti = None;
        let mut accepted_at = None;
        let mut estimated_completion = None;
        let mut executor_sig = None;
        let mut rejected_at = None;
        let mut reason = None;
        let mut detail = None;
        let mut proof: Option<ExecutionProof> = None;
        let mut data_location: Option<DataLocation> = None;
        let mut failed_at = None;
        let mut partial_proof = None;

        decode_map(dec, |key, dec| {
            match key {
                "type" | "status" => variant = Some(dec.str()?.to_string()),
                "task_jti" => task_jti = Some(dec.str()?.to_string()),
                "accepted_at" => accepted_at = Some(dec.u64()?),
                "estimated_completion" => estimated_completion = Some(dec.u64()?),
                "executor_sig" => executor_sig = Some(dec.bytes()?.to_vec()),
                "rejected_at" => rejected_at = Some(dec.u64()?),
                "reason" => reason = Some(dec.str()?.to_string()),
                "detail" => detail = Some(dec.str()?.to_string()),
                "proof" => proof = Some(ExecutionProof::decode_cbor_from(dec)?),
                "data_location" => data_location = Some(DataLocation::decode_from(dec)?),
                "failed_at" => failed_at = Some(dec.u64()?),
                "partial_proof" => partial_proof = Some(ExecutionProof::decode_cbor_from(dec)?),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        let variant = variant.ok_or_else(|| ProtocolError::new("task response missing type"))?;
        match variant.as_str() {
            "ACCEPTED" => Ok(TaskResponse::Accepted(TaskAccepted {
                task_jti: task_jti.ok_or_else(|| ProtocolError::new("accepted missing task_jti"))?,
                accepted_at: accepted_at
                    .ok_or_else(|| ProtocolError::new("accepted missing accepted_at"))?,
                estimated_completion: estimated_completion
                    .ok_or_else(|| ProtocolError::new("accepted missing estimated_completion"))?,
                executor_sig: executor_sig
                    .ok_or_else(|| ProtocolError::new("accepted missing executor_sig"))?,
            })),
            "REJECTED" => Ok(TaskResponse::Rejected(TaskRejected {
                task_jti: task_jti.ok_or_else(|| ProtocolError::new("rejected missing task_jti"))?,
                rejected_at: rejected_at
                    .ok_or_else(|| ProtocolError::new("rejected missing rejected_at"))?,
                reason: reason.ok_or_else(|| ProtocolError::new("rejected missing reason"))?,
                detail,
                executor_sig: executor_sig
                    .ok_or_else(|| ProtocolError::new("rejected missing executor_sig"))?,
            })),
            "COMPLETED" => Ok(TaskResponse::Completed(TaskCompleted {
                task_jti: task_jti.ok_or_else(|| ProtocolError::new("completed missing task_jti"))?,
                proof: proof.ok_or_else(|| ProtocolError::new("completed missing proof"))?,
                data_location,
            })),
            "FAILED" => Ok(TaskResponse::Failed(TaskFailed {
                task_jti: task_jti.ok_or_else(|| ProtocolError::new("failed missing task_jti"))?,
                failed_at: failed_at
                    .ok_or_else(|| ProtocolError::new("failed missing failed_at"))?,
                reason: reason.ok_or_else(|| ProtocolError::new("failed missing reason"))?,
                detail,
                partial_proof,
                executor_sig: executor_sig
                    .ok_or_else(|| ProtocolError::new("failed missing executor_sig"))?,
            })),
            _ => Err(ProtocolError::new("task response unknown type")),
        }
    }
}

impl TaskAccepted {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        enc.map(5)?;
        enc.str("type")?;
        enc.str("ACCEPTED")?;
        enc.str("task_jti")?;
        enc.str(&self.task_jti)?;
        enc.str("accepted_at")?;
        enc.u64(self.accepted_at)?;
        enc.str("estimated_completion")?;
        enc.u64(self.estimated_completion)?;
        enc.str("executor_sig")?;
        enc.bytes(&self.executor_sig)?;
        Ok(())
    }

    fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        self.encode_into(&mut enc)?;
        Ok(buf)
    }
}

impl TaskRejected {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 5;
        if self.detail.is_some() {
            len += 1;
        }
        enc.map(len)?;
        enc.str("type")?;
        enc.str("REJECTED")?;
        enc.str("task_jti")?;
        enc.str(&self.task_jti)?;
        enc.str("rejected_at")?;
        enc.u64(self.rejected_at)?;
        enc.str("reason")?;
        enc.str(&self.reason)?;
        if let Some(detail) = &self.detail {
            enc.str("detail")?;
            enc.str(detail)?;
        }
        enc.str("executor_sig")?;
        enc.bytes(&self.executor_sig)?;
        Ok(())
    }

    fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        self.encode_into(&mut enc)?;
        Ok(buf)
    }
}

impl DataLocation {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 1;
        if self.relay_satellite.is_some() {
            len += 1;
        }
        if self.ground_station.is_some() {
            len += 1;
        }
        if self.estimated_delivery.is_some() {
            len += 1;
        }
        enc.map(len)?;
        enc.str("method")?;
        enc.str(&self.method)?;
        if let Some(val) = &self.relay_satellite {
            enc.str("relay_satellite")?;
            enc.str(val)?;
        }
        if let Some(val) = &self.ground_station {
            enc.str("ground_station")?;
            enc.str(val)?;
        }
        if let Some(val) = self.estimated_delivery {
            enc.str("estimated_delivery")?;
            enc.u64(val)?;
        }
        Ok(())
    }

    fn decode_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut method = None;
        let mut relay_satellite = None;
        let mut ground_station = None;
        let mut estimated_delivery = None;
        decode_map(dec, |key, dec| {
            match key {
                "method" => method = Some(dec.str()?.to_string()),
                "relay_satellite" => relay_satellite = Some(dec.str()?.to_string()),
                "ground_station" => ground_station = Some(dec.str()?.to_string()),
                "estimated_delivery" => estimated_delivery = Some(dec.u64()?),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;
        Ok(DataLocation {
            method: method.ok_or_else(|| ProtocolError::new("data_location missing method"))?,
            relay_satellite,
            ground_station,
            estimated_delivery,
        })
    }
}

impl TaskCompleted {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 3;
        if self.data_location.is_some() {
            len += 1;
        }
        enc.map(len)?;
        enc.str("type")?;
        enc.str("COMPLETED")?;
        enc.str("task_jti")?;
        enc.str(&self.task_jti)?;
        enc.str("proof")?;
        self.proof.encode_into(enc)?;
        if let Some(loc) = &self.data_location {
            enc.str("data_location")?;
            loc.encode_into(enc)?;
        }
        Ok(())
    }

    fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        self.encode_into(&mut enc)?;
        Ok(buf)
    }
}

impl TaskFailed {
    fn encode_into(&self, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
        let mut len = 5;
        if self.detail.is_some() {
            len += 1;
        }
        if self.partial_proof.is_some() {
            len += 1;
        }
        enc.map(len)?;
        enc.str("type")?;
        enc.str("FAILED")?;
        enc.str("task_jti")?;
        enc.str(&self.task_jti)?;
        enc.str("failed_at")?;
        enc.u64(self.failed_at)?;
        enc.str("reason")?;
        enc.str(&self.reason)?;
        if let Some(detail) = &self.detail {
            enc.str("detail")?;
            enc.str(detail)?;
        }
        if let Some(proof) = &self.partial_proof {
            enc.str("partial_proof")?;
            proof.encode_into(enc)?;
        }
        enc.str("executor_sig")?;
        enc.bytes(&self.executor_sig)?;
        Ok(())
    }

    fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        self.encode_into(&mut enc)?;
        Ok(buf)
    }
}

impl MessageType {
    fn as_str(&self) -> &'static str {
        match self {
            MessageType::TaskRequest => "TASK_REQUEST",
            MessageType::TaskResponse => "TASK_RESPONSE",
            MessageType::Proof => "PROOF",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "TASK_REQUEST" => Some(MessageType::TaskRequest),
            "TASK_RESPONSE" => Some(MessageType::TaskResponse),
            "PROOF" => Some(MessageType::Proof),
            _ => None,
        }
    }
}

impl IslScapMessage {
    pub fn encode_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = Vec::new();
        let mut enc = Encoder::new(&mut buf);
        let mut len = 7;
        if self.hmac.is_some() {
            len += 1;
        }
        enc.map(len)?;
        enc.str("version")?;
        enc.u64(self.version)?;
        enc.str("msg_type")?;
        enc.str(self.msg_type.as_str())?;
        enc.str("sender")?;
        enc.str(&self.sender)?;
        enc.str("recipient")?;
        enc.str(&self.recipient)?;
        enc.str("sequence")?;
        enc.u64(self.sequence)?;
        enc.str("timestamp")?;
        enc.u64(self.timestamp)?;
        enc.str("payload")?;
        encode_payload(&self.payload, &mut enc)?;
        if let Some(hmac) = &self.hmac {
            enc.str("hmac")?;
            enc.bytes(hmac)?;
        }
        Ok(buf)
    }

    pub fn decode_cbor(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut dec = Decoder::new(bytes);
        let mut version = None;
        let mut msg_type: Option<MessageType> = None;
        let mut sender = None;
        let mut recipient = None;
        let mut sequence = None;
        let mut timestamp = None;
        let mut payload: Option<ScapPayload> = None;
        let mut hmac = None;

        decode_map(&mut dec, |key, dec| {
            match key {
                "version" => version = Some(dec.u64()?),
                "msg_type" => {
                    let value = dec.str()?;
                    msg_type = MessageType::from_str(value);
                }
                "sender" => sender = Some(dec.str()?.to_string()),
                "recipient" => recipient = Some(dec.str()?.to_string()),
                "sequence" => sequence = Some(dec.u64()?),
                "timestamp" => timestamp = Some(dec.u64()?),
                "payload" => {
                    let msg_type_val =
                        msg_type.ok_or_else(|| ProtocolError::new("isl missing msg_type"))?;
                    payload = Some(decode_payload(msg_type_val, dec)?);
                }
                "hmac" => hmac = Some(dec.bytes()?.to_vec()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(IslScapMessage {
            version: version.ok_or_else(|| ProtocolError::new("isl missing version"))?,
            msg_type: msg_type.ok_or_else(|| ProtocolError::new("isl missing msg_type"))?,
            sender: sender.ok_or_else(|| ProtocolError::new("isl missing sender"))?,
            recipient: recipient.ok_or_else(|| ProtocolError::new("isl missing recipient"))?,
            sequence: sequence.ok_or_else(|| ProtocolError::new("isl missing sequence"))?,
            timestamp: timestamp.ok_or_else(|| ProtocolError::new("isl missing timestamp"))?,
            payload: payload.ok_or_else(|| ProtocolError::new("isl missing payload"))?,
            hmac,
        })
    }
}

fn encode_payload(payload: &ScapPayload, enc: &mut Encoder<&mut Vec<u8>>) -> Result<(), ProtocolError> {
    match payload {
        ScapPayload::TaskRequest(req) => req.encode_into(enc)?,
        ScapPayload::TaskResponse(resp) => match resp {
            TaskResponse::Accepted(accepted) => accepted.encode_into(enc)?,
            TaskResponse::Rejected(rejected) => rejected.encode_into(enc)?,
            TaskResponse::Completed(completed) => completed.encode_into(enc)?,
            TaskResponse::Failed(failed) => failed.encode_into(enc)?,
        },
        ScapPayload::Proof(proof) => proof.encode_into(enc)?,
    }
    Ok(())
}

fn decode_payload(
    msg_type: MessageType,
    dec: &mut Decoder<'_>,
) -> Result<ScapPayload, ProtocolError> {
    match msg_type {
        MessageType::TaskRequest => Ok(ScapPayload::TaskRequest(BoundTaskRequest::decode_cbor_from(dec)?)),
        MessageType::TaskResponse => Ok(ScapPayload::TaskResponse(TaskResponse::decode_from(dec)?)),
        MessageType::Proof => Ok(ScapPayload::Proof(ExecutionProof::decode_cbor_from(dec)?)),
    }
}

impl BoundTaskRequest {
    fn decode_cbor_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut capability_token = None;
        let mut payment_hash = None;
        let mut payment_amount_msat = None;
        let mut htlc_timeout_blocks = None;
        let mut binding_sig = None;

        decode_map(dec, |key, dec| {
            match key {
                "capability_token" => capability_token = Some(dec.bytes()?.to_vec()),
                "payment_hash" => payment_hash = Some(dec.bytes()?.to_vec()),
                "payment_amount_msat" => payment_amount_msat = Some(dec.u64()?),
                "htlc_timeout_blocks" => htlc_timeout_blocks = Some(decode_u32(dec)?),
                "binding_sig" => binding_sig = Some(dec.bytes()?.to_vec()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(BoundTaskRequest {
            capability_token: capability_token
                .ok_or_else(|| ProtocolError::new("bound task missing capability_token"))?,
            payment_hash: payment_hash
                .ok_or_else(|| ProtocolError::new("bound task missing payment_hash"))?,
            payment_amount_msat: payment_amount_msat
                .ok_or_else(|| ProtocolError::new("bound task missing payment_amount_msat"))?,
            htlc_timeout_blocks: htlc_timeout_blocks
                .ok_or_else(|| ProtocolError::new("bound task missing htlc_timeout_blocks"))?,
            binding_sig: binding_sig
                .ok_or_else(|| ProtocolError::new("bound task missing binding_sig"))?,
        })
    }
}

impl ExecutionProof {
    fn decode_cbor_from(dec: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let mut task_jti = None;
        let mut payment_hash = None;
        let mut output_hash = None;
        let mut execution_timestamp = None;
        let mut output_metadata = None;
        let mut executor_sig = None;

        decode_map(dec, |key, dec| {
            match key {
                "task_jti" => task_jti = Some(dec.str()?.to_string()),
                "payment_hash" => payment_hash = Some(dec.bytes()?.to_vec()),
                "output_hash" => output_hash = Some(dec.bytes()?.to_vec()),
                "execution_timestamp" => execution_timestamp = Some(dec.u64()?),
                "output_metadata" => output_metadata = Some(OutputMetadata::decode_from(dec)?),
                "executor_sig" => executor_sig = Some(dec.bytes()?.to_vec()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })?;

        Ok(ExecutionProof {
            task_jti: task_jti.ok_or_else(|| ProtocolError::new("proof missing task_jti"))?,
            payment_hash: payment_hash
                .ok_or_else(|| ProtocolError::new("proof missing payment_hash"))?,
            output_hash: output_hash
                .ok_or_else(|| ProtocolError::new("proof missing output_hash"))?,
            execution_timestamp: execution_timestamp
                .ok_or_else(|| ProtocolError::new("proof missing execution_timestamp"))?,
            output_metadata,
            executor_sig: executor_sig
                .ok_or_else(|| ProtocolError::new("proof missing executor_sig"))?,
        })
    }
}

fn decode_map<F>(dec: &mut Decoder<'_>, mut f: F) -> Result<(), ProtocolError>
where
    F: FnMut(&str, &mut Decoder<'_>) -> Result<(), ProtocolError>,
{
    let len = dec.map().map_err(to_err)?;
    match len {
        Some(count) => {
            for _ in 0..count {
                let key = dec.str().map_err(to_err)?;
                f(key, dec)?;
            }
        }
        None => loop {
            if dec.datatype().map_err(to_err)? == Type::Break {
                dec.skip().map_err(to_err)?;
                break;
            }
            let key = dec.str().map_err(to_err)?;
            f(key, dec)?;
        },
    }
    Ok(())
}

fn decode_u32(dec: &mut Decoder<'_>) -> Result<u32, ProtocolError> {
    let value = dec.u64().map_err(to_err)?;
    if value > u32::MAX as u64 {
        return Err(ProtocolError::new("u32 overflow"));
    }
    Ok(value as u32)
}

fn decode_f64(dec: &mut Decoder<'_>) -> Result<f64, ProtocolError> {
    match dec.datatype().map_err(to_err)? {
        Type::F64 => Ok(dec.f64().map_err(to_err)?),
        Type::F32 => Ok(dec.f32().map_err(to_err)? as f64),
        Type::U64 => Ok(dec.u64().map_err(to_err)? as f64),
        Type::U32 => Ok(dec.u32().map_err(to_err)? as f64),
        Type::I64 => Ok(dec.i64().map_err(to_err)? as f64),
        Type::I32 => Ok(dec.i32().map_err(to_err)? as f64),
        _ => Err(ProtocolError::new("expected numeric type")),
    }
}

fn to_err<E: core::fmt::Display>(err: E) -> ProtocolError {
    ProtocolError::new(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_to_bytes;
    use std::path::PathBuf;

    fn spec_root() -> PathBuf {
        let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.push("..");
        root.push("..");
        root.push("..");
        root.push("scrap-master");
        root
    }

    fn examples_dir() -> PathBuf {
        spec_root().join("schemas").join("examples")
    }

    fn read_bytes(path: &PathBuf) -> Vec<u8> {
        std::fs::read(path).expect("read fixture")
    }

    fn read_json(path: &PathBuf) -> serde_json::Value {
        let raw = std::fs::read_to_string(path).expect("read json");
        serde_json::from_str(&raw).expect("parse json")
    }

    fn decode_dispute_fields(bytes: &[u8]) -> (String, Vec<u8>) {
        let mut dec = Decoder::new(bytes);
        let mut task_jti: Option<String> = None;
        let mut payment_hash: Option<Vec<u8>> = None;
        decode_map(&mut dec, |key, dec| {
            match key {
                "task_jti" => task_jti = Some(dec.str()?.to_string()),
                "payment_hash" => payment_hash = Some(dec.bytes()?.to_vec()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })
        .expect("decode dispute");

        (
            task_jti.expect("task_jti"),
            payment_hash.expect("payment_hash"),
        )
    }

    fn decode_heartbeat_fields(bytes: &[u8]) -> (String, u64) {
        let mut dec = Decoder::new(bytes);
        let mut sender: Option<String> = None;
        let mut pending_htlcs: Option<u64> = None;
        decode_map(&mut dec, |key, dec| {
            match key {
                "sender" => sender = Some(dec.str()?.to_string()),
                "pending_htlcs" => pending_htlcs = Some(dec.u64()?),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })
        .expect("decode heartbeat");

        (
            sender.expect("sender"),
            pending_htlcs.expect("pending_htlcs"),
        )
    }

    fn decode_lightning_bolt_payload(bytes: &[u8]) -> Vec<u8> {
        let mut dec = Decoder::new(bytes);
        let mut bolt_payload: Option<Vec<u8>> = None;
        decode_map(&mut dec, |key, dec| {
            match key {
                "bolt_payload" => bolt_payload = Some(dec.bytes()?.to_vec()),
                _ => {
                    dec.skip()?;
                }
            }
            Ok(())
        })
        .expect("decode lightning wrapper");

        bolt_payload.expect("bolt_payload")
    }

    #[test]
    fn decode_capability_token_fixture() {
        let cbor_path = examples_dir().join("capability_token.cbor");
        let json_path = examples_dir().join("capability_token.json");
        let token = SatCapToken::decode_cbor(&read_bytes(&cbor_path)).expect("decode cbor");
        let json = read_json(&json_path);

        assert_eq!(token.header.alg, json["header"]["alg"].as_str().unwrap());
        assert_eq!(token.header.typ, json["header"]["typ"].as_str().unwrap());
        assert_eq!(
            token.header.enc.as_deref(),
            json["header"]["enc"].as_str()
        );
        assert_eq!(token.payload.iss, json["payload"]["iss"].as_str().unwrap());
        assert_eq!(token.payload.sub, json["payload"]["sub"].as_str().unwrap());
        assert_eq!(token.payload.aud, json["payload"]["aud"].as_str().unwrap());
        assert_eq!(token.payload.iat, json["payload"]["iat"].as_u64().unwrap());
        assert_eq!(token.payload.exp, json["payload"]["exp"].as_u64().unwrap());
        assert_eq!(token.payload.jti, json["payload"]["jti"].as_str().unwrap());
        let caps = json["payload"]["cap"].as_array().unwrap();
        assert_eq!(token.payload.cap.len(), caps.len());
        let sig_hex = json["signature"].as_str().unwrap();
        let sig_bytes = hex_to_bytes(sig_hex).expect("sig hex");
        assert_eq!(token.signature, sig_bytes);
    }

    #[test]
    fn decode_delegation_token_fixture() {
        let cbor_path = examples_dir().join("delegation_token.cbor");
        let json_path = examples_dir().join("delegation_token.json");
        let token = SatCapToken::decode_cbor(&read_bytes(&cbor_path)).expect("decode cbor");
        let json = read_json(&json_path);

        assert_eq!(token.header.typ, json["header"]["typ"].as_str().unwrap());
        assert_eq!(token.header.chn, json["header"]["chn"].as_u64().map(|v| v as u32));
        assert_eq!(token.payload.prf.as_deref(), json["payload"]["prf"].as_str());
        assert_eq!(token.payload.jti, json["payload"]["jti"].as_str().unwrap());
    }

    #[test]
    fn decode_bound_task_request_fixture() {
        let cbor_path = examples_dir().join("bound_task_request.cbor");
        let json_path = examples_dir().join("bound_task_request.json");
        let request = BoundTaskRequest::decode_cbor(&read_bytes(&cbor_path)).expect("decode");
        let json = read_json(&json_path);

        let cap_hex = json["capability_token"].as_str().unwrap();
        let cap_bytes = hex_to_bytes(cap_hex).expect("cap hex");
        assert_eq!(request.capability_token, cap_bytes);
        let payment_hash_hex = json["payment_hash"].as_str().unwrap();
        let payment_hash = hex_to_bytes(payment_hash_hex).expect("payment hash hex");
        assert_eq!(request.payment_hash, payment_hash);
        assert_eq!(
            request.payment_amount_msat,
            json["payment_amount_msat"].as_u64().unwrap()
        );
        assert_eq!(
            request.htlc_timeout_blocks,
            json["htlc_timeout_blocks"].as_u64().unwrap() as u32
        );
        let sig_hex = json["binding_sig"].as_str().unwrap();
        let sig_bytes = hex_to_bytes(sig_hex).expect("binding sig");
        assert_eq!(request.binding_sig, sig_bytes);
    }

    #[test]
    fn decode_execution_proof_fixture() {
        let cbor_path = examples_dir().join("execution_proof.cbor");
        let json_path = examples_dir().join("execution_proof.json");
        let proof = ExecutionProof::decode_cbor(&read_bytes(&cbor_path)).expect("decode");
        let json = read_json(&json_path);

        assert_eq!(proof.task_jti, json["task_jti"].as_str().unwrap());
        let payment_hash = hex_to_bytes(json["payment_hash"].as_str().unwrap()).unwrap();
        assert_eq!(proof.payment_hash, payment_hash);
        let output_hash = hex_to_bytes(json["output_hash"].as_str().unwrap()).unwrap();
        assert_eq!(proof.output_hash, output_hash);
        assert_eq!(
            proof.execution_timestamp,
            json["execution_timestamp"].as_u64().unwrap()
        );
        let sig_hex = json["executor_sig"].as_str().unwrap();
        let sig_bytes = hex_to_bytes(sig_hex).unwrap();
        assert_eq!(proof.executor_sig, sig_bytes);
        if let Some(meta) = &proof.output_metadata {
            let json_meta = &json["output_metadata"];
            assert_eq!(
                meta.data_size_bytes,
                json_meta["data_size_bytes"].as_u64()
            );
            assert_eq!(
                meta.data_format.as_deref(),
                json_meta["data_format"].as_str()
            );
            assert_eq!(
                meta.coverage_km2,
                json_meta["coverage_km2"].as_f64()
            );
            assert_eq!(
                meta.sensor_mode.as_deref(),
                json_meta["sensor_mode"].as_str()
            );
        }
    }

    #[test]
    fn decode_isl_tasklib_fixture() {
        let cbor_path = examples_dir().join("isl_tasklib_message.cbor");
        let json_path = examples_dir().join("isl_tasklib_message.json");
        let isl = IslScapMessage::decode_cbor(&read_bytes(&cbor_path)).expect("decode");
        let json = read_json(&json_path);

        assert_eq!(isl.version, json["version"].as_u64().unwrap());
        assert_eq!(isl.sender, json["sender"].as_str().unwrap());
        assert_eq!(isl.recipient, json["recipient"].as_str().unwrap());
        assert_eq!(isl.sequence, json["sequence"].as_u64().unwrap());
        assert_eq!(isl.timestamp, json["timestamp"].as_u64().unwrap());
        let hmac_hex = json["hmac"].as_str().unwrap();
        let hmac_bytes = hex_to_bytes(hmac_hex).unwrap();
        assert_eq!(isl.hmac, Some(hmac_bytes));
        if let ScapPayload::TaskRequest(req) = isl.payload {
            let payload = &json["payload"];
            let cap_hex = payload["capability_token"].as_str().unwrap();
            let cap_bytes = hex_to_bytes(cap_hex).unwrap();
            assert_eq!(req.capability_token, cap_bytes);
        } else {
            panic!("expected task request payload");
        }
    }

    #[test]
    fn decode_dispute_and_heartbeat_fixtures() {
        let dispute_cbor = read_bytes(&examples_dir().join("dispute_message.cbor"));
        let dispute_json = read_json(&examples_dir().join("dispute_message.json"));
        let (task_jti, payment_hash) = decode_dispute_fields(&dispute_cbor);
        assert_eq!(task_jti, dispute_json["task_jti"].as_str().unwrap());
        let expected_hash = hex_to_bytes(dispute_json["payment_hash"].as_str().unwrap()).unwrap();
        assert_eq!(payment_hash, expected_hash);

        let heartbeat_cbor = read_bytes(&examples_dir().join("heartbeat.cbor"));
        let heartbeat_json = read_json(&examples_dir().join("heartbeat.json"));
        let (sender, pending) = decode_heartbeat_fields(&heartbeat_cbor);
        assert_eq!(sender, heartbeat_json["sender"].as_str().unwrap());
        assert_eq!(pending, heartbeat_json["pending_htlcs"].as_u64().unwrap());
    }

    #[test]
    fn decode_other_cbor_fixtures() {
        let accepted =
            TaskResponse::decode_cbor(&read_bytes(&examples_dir().join("task_accepted.cbor")))
                .expect("decode accepted");
        assert!(matches!(accepted, TaskResponse::Accepted(_)));

        let rejected =
            TaskResponse::decode_cbor(&read_bytes(&examples_dir().join("task_rejected.cbor")))
                .expect("decode rejected");
        assert!(matches!(rejected, TaskResponse::Rejected(_)));

        let completed =
            TaskResponse::decode_cbor(&read_bytes(&examples_dir().join("task_completed.cbor")))
                .expect("decode completed");
        assert!(matches!(completed, TaskResponse::Completed(_)));

        let failed =
            TaskResponse::decode_cbor(&read_bytes(&examples_dir().join("task_failed.cbor")))
                .expect("decode failed");
        assert!(matches!(failed, TaskResponse::Failed(_)));

        let lightning_cbor = read_bytes(&examples_dir().join("lightning_wrapper.cbor"));
        let bolt_payload = decode_lightning_bolt_payload(&lightning_cbor);
        assert!(!bolt_payload.is_empty());
    }

    #[test]
    fn roundtrip_basic_structs() {
        let token = SatCapToken {
            header: CapHeader {
                alg: "ES256K".to_string(),
                typ: "SAT-CAP".to_string(),
                enc: Some("CBOR".to_string()),
                chn: None,
            },
            payload: CapPayload {
                iss: "OPERATOR".to_string(),
                sub: "SUBJECT".to_string(),
                aud: "AUDIENCE".to_string(),
                iat: 1,
                exp: 2,
                jti: "token-1".to_string(),
                cap: vec!["cmd:imaging:msi".to_string()],
                cns: None,
                prf: None,
                cmd_pub: None,
            },
            signature: vec![1, 2, 3],
        };
        let bytes = token.encode_cbor().expect("encode token");
        let decoded = SatCapToken::decode_cbor(&bytes).expect("decode token");
        assert_eq!(token, decoded);

        let proof = ExecutionProof {
            task_jti: "task-1".to_string(),
            payment_hash: vec![0u8; 32],
            output_hash: vec![1u8; 32],
            execution_timestamp: 10,
            output_metadata: None,
            executor_sig: vec![2u8; 70],
        };
        let proof_bytes = proof.encode_cbor().expect("encode proof");
        let decoded_proof = ExecutionProof::decode_cbor(&proof_bytes).expect("decode proof");
        assert_eq!(proof, decoded_proof);
    }
}
