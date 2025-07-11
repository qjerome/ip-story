#![deny(unused_imports)]

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet},
    env,
    ffi::OsStr,
    net::IpAddr,
    path::PathBuf,
    sync::Arc,
};

use api::{ApiData, ApiResult};
use chrono::Utc;
use redis::{Client, Commands, RedisError};
use rocket::{
    FromFormField, State, delete, error, get, http::ContentType, post, put, request::FromParam,
    routes, serde::json::Json,
};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use url::Url;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

mod api;

use api::ApiResponse;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Owner {
    name: String,
    address: Option<String>,
    country: Option<String>,
    abuse: Option<String>,
    phone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TicketId {
    Id(u64),
    Uuid(Uuid),
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct MispEvent {
    server: Option<Url>,
    uuid: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Ticket {
    server: Option<Url>,
    id: TicketId,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Data {
    Owner(Owner),
    Asn(u64),
    MispEvent(MispEvent),
    Ticket(Ticket),
    Vulnerable(String),
    Text(String),
    Json(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize, FromFormField, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SearchOrder {
    Asc,
    Desc,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, FromFormField, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DataKind {
    Owner,
    Asn,
    // for FromFormField
    #[field(value = "misp-event")]
    MispEvent,
    Ticket,
    Vulnerable,
    Text,
    Json,
}

impl<'r> FromParam<'r> for DataKind {
    type Error = &'r str;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        match param {
            "owner" => Ok(DataKind::Owner),
            "asn" => Ok(DataKind::Asn),
            "misp-event" => Ok(DataKind::MispEvent),
            "ticket" => Ok(DataKind::Ticket),
            "vulnerable" => Ok(DataKind::Vulnerable),
            "text" => Ok(DataKind::Text),
            "json" => Ok(DataKind::Json),
            _ => Err(param),
        }
    }
}

impl Data {
    fn kind(&self) -> DataKind {
        match self {
            Self::Owner(_) => DataKind::Owner,
            Self::Asn(_) => DataKind::Asn,
            Self::MispEvent(_) => DataKind::MispEvent,
            Self::Ticket(_) => DataKind::Ticket,
            Self::Vulnerable(_) => DataKind::Vulnerable,
            Self::Text(_) => DataKind::Text,
            Self::Json(_) => DataKind::Json,
        }
    }
}

#[derive(Hash, Debug, PartialEq, Eq, Clone, ToSchema)]
struct Tag(String);

impl From<String> for Tag {
    fn from(value: String) -> Self {
        Tag(value.to_ascii_lowercase())
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Tag(String::deserialize(deserializer)?))
    }
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.to_ascii_lowercase().serialize(serializer)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Entry {
    uuid: Option<Uuid>,
    description: Option<String>,
    /// Creation timestamp
    ctime: Option<chrono::DateTime<Utc>>,
    /// Modification timestamp
    mtime: Option<chrono::DateTime<Utc>>,
    tags: Option<HashSet<Tag>>,
    data: Data,
}

type History = BTreeMap<chrono::DateTime<Utc>, Entry>;

#[derive(Debug, Serialize, Deserialize)]
struct IpStory {
    ip: IpAddr,
    history: History,
}

impl IpStory {
    fn new(ip: IpAddr) -> Self {
        IpStory {
            ip,
            history: BTreeMap::new(),
        }
    }
}

const API_MOUNTPOINT: &str = "/api";
const MAP_NAME: &str = "ip-story";

fn connect_to_redis() -> anyhow::Result<redis::Client> {
    // Get the Redis URL from the environment variable
    let redis_url = env::var("REDIS_URL")?;

    // Create a Redis client
    let client = Client::open(redis_url)?;

    Ok(client)
}

fn get_hip(ip: IpAddr, client: &mut redis::Client) -> Result<IpStory, RedisError> {
    let s: String = client.hget(MAP_NAME, ip.to_string())?;
    Ok(serde_json::from_str(&s).unwrap())
}

fn hip_exists(ip: IpAddr, client: &mut redis::Client) -> Result<bool, RedisError> {
    let s = client.hexists(MAP_NAME, ip.to_string())?;
    Ok(s)
}

fn store_hip(hip: IpStory, client: &mut redis::Client) -> Result<(), RedisError> {
    client.hset(
        MAP_NAME,
        hip.ip.to_string(),
        serde_json::to_string(&hip).unwrap(),
    )
}

#[utoipa::path(
    context_path = API_MOUNTPOINT,
    params(
        ("ip" = String, Path, description = "The IP address to add or update"),
    ),
    responses(
        (status = 200, description = "IP address processed successfully", body = ApiResponse<String>, content_type = "application/json"),
    ),
    tag = "IP Management",
    description = "Adds a new IP address to the database if it does not already exist. Returns an ApiResponse with the IP address or an error message."
)]
#[put("/ip/<ip>")]
async fn ip_new(ip: IpAddr, db: &State<Arc<Mutex<redis::Client>>>) -> ApiResult<IpAddr> {
    let mut db = db.lock().await;
    if !hip_exists(ip, &mut db)
        .inspect_err(|e| error!("failed to insert new ip: {e}"))
        .map_err(|_| api_error!("failed to insert new ip"))?
    {
        store_hip(IpStory::new(ip), &mut db)
            .inspect_err(|e| error!("failed to insert new ip: {e}"))
            .map_err(|_| api_error!("failed to insert new ip"))?;
    }
    Ok(ApiData::Some(ip))
}

#[utoipa::path(
    context_path = API_MOUNTPOINT,
    request_body = Entry,
    params(
        ("ip" = String, Path, description = "The IP address"),
    ),
    responses(
        (status = 200, description = "Entry addition response", body = ApiResponse<bool>, content_type = "application/json"),
    ),
    tag = "IP Management",
    description = "Adds a new entry associated with an IP address. Returns an ApiResponse with a boolean indicating success or an error message."
)]
#[post("/ip/<ip>/entry", data = "<entry>")]
async fn ip_add_entry(
    ip: IpAddr,
    entry: Json<Entry>,
    db: &State<Arc<Mutex<redis::Client>>>,
) -> ApiResult<bool> {
    let mut db = db.lock().await;

    let mut ipst = get_hip(ip, &mut db)
        .inspect_err(|e| error!("failed to get data from db: {e}"))
        .map_err(|_| api_error!("failed to get data from db"))?;

    // we append entry
    let mut entry = entry.0;
    // we must create a new uuid
    entry.uuid = Some(Uuid::new_v4());
    let timestamp = entry.ctime.get_or_insert_with(Utc::now);

    if ipst.history.contains_key(timestamp) {
        return Err(api_error!(
            "an entry with this timestamp is already present"
        ));
    }

    ipst.history.insert(*timestamp, entry);

    store_hip(ipst, &mut db)
        .inspect_err(|e| error!("failed to insert new ip: {e}"))
        .map_err(|_| api_error!("failed to insert new ip"))?;

    Ok(ApiData::Some(true))
}

#[utoipa::path(
    context_path = API_MOUNTPOINT,
    request_body = Entry,
    params(
        ("ip" = String, Path, description = "The IP address"),
    ),
    responses(
        (status = 200, description = "Entry update response", body = ApiResponse<bool>, content_type = "application/json"),
    ),
    tag = "IP Management",
    description = "Updates an existing entry associated with an IP address. Returns an ApiResponse with a boolean indicating success or an error message."
)]
#[post("/ip/<ip>/entry/update", data = "<entry>")]
async fn ip_update_entry(
    ip: IpAddr,
    entry: Json<Entry>,
    db: &State<Arc<Mutex<redis::Client>>>,
) -> ApiResult<bool> {
    let mut db = db.lock().await;

    let mut ipst = get_hip(ip, &mut db)
        .inspect_err(|e| error!("failed to get data from db: {e}"))
        .map_err(|_| api_error!("failed to get data from db"))?;

    let mut entry = entry.0;

    // we search the key of an existing entry (by its uuid)
    // searching by UUID allows changing the creation time
    // without delete + create
    let Some(key) = ipst
        .history
        .iter()
        .find(|(_, v)| v.uuid == entry.uuid)
        .map(|(k, _)| k)
    else {
        return Ok(ApiData::Some(false));
    };

    entry.mtime = Some(Utc::now());
    ipst.history.insert(*key, entry);

    store_hip(ipst, &mut db)
        .inspect_err(|e| error!("failed to insert new ip: {e}"))
        .map_err(|_| api_error!("failed to insert new ip"))?;

    Ok(ApiData::Some(true))
}

#[utoipa::path(
    context_path = API_MOUNTPOINT,
    params(
        ("ip" = String, Path, description = "The IP address"),
        ("kind" = Option<DataKind>, Query, description = "The kind of data to search for"),
        ("limit" = Option<usize>, Query, description = "The maximum number of entries to return"),
        ("offset" = Option<usize>, Query, description = "The number of entries to skip"),
        ("order" = Option<SearchOrder>, Query, description = "The order in which to return the entries")
    ),
    responses(
        (status = 200, description = "Entries retrieved successfully", body = ApiResponse<Vec<Entry>>, content_type = "application/json"),
    ),
    tag = "IP Management",
    description = "Searches for entries associated with an IP address based on the given criteria."
)]
#[get("/ip/<ip>/entry/search?<kind>&<offset>&<limit>&<order>")]
async fn ip_search_entry(
    ip: IpAddr,
    kind: Option<DataKind>,
    limit: Option<usize>,
    offset: Option<usize>,
    order: Option<SearchOrder>,
    db: &State<Arc<Mutex<redis::Client>>>,
) -> ApiResult<Vec<Entry>> {
    let mut db = db.lock().await;

    let limit = limit.unwrap_or(usize::MAX);
    let offset = offset.unwrap_or_default();
    let order = order.unwrap_or(SearchOrder::Asc);

    let ipst = get_hip(ip, &mut db).map_err(|_| api_error!("failed to get data from db"))?;

    let iter: Box<dyn Iterator<Item = _>> = match order {
        SearchOrder::Asc => Box::new(ipst.history.iter()),
        SearchOrder::Desc => Box::new(ipst.history.iter().rev()),
    };

    let hist: Vec<Entry> = iter
        // filter by kind
        .filter(|(_, e)| {
            if let Some(kind) = &kind {
                &e.data.kind() == kind
            } else {
                true
            }
        })
        // start at offset
        .skip(offset)
        // take only limit
        .take(limit)
        .map(|(_, e)| e.clone())
        .collect();

    Ok(ApiData::Some(hist))
}

#[utoipa::path(
    context_path = API_MOUNTPOINT,
    params(
        ("ip" = String, Path, description = "The IP address"),
        ("uuid" = Option<Uuid>, Path, description = "The UUID of the entry to delete")
    ),
    responses(
        (status = 200, description = "Entry deletion response", body = ApiResponse<Entry>, content_type = "application/json"),
    ),
    tag = "IP Management",
    description = "Deletes an entry associated with an IP address. Returns an ApiResponse with an optional deleted entry data or an error message."
)]
#[delete("/ip/<ip>/entry/<uuid>")]
async fn ip_del_entry(
    ip: IpAddr,
    uuid: Option<Uuid>,
    db: &State<Arc<Mutex<redis::Client>>>,
) -> ApiResult<Entry> {
    let mut db = db.lock().await;

    let mut ipst = get_hip(ip, &mut db).map_err(|_| api_error!("failed to get data from db"))?;

    let Some(key) = ipst
        .history
        .iter()
        .find(|(_, v)| v.uuid == uuid)
        .map(|(k, _)| k)
        .cloned()
    else {
        return Ok(ApiData::None);
    };

    Ok(ApiData::from(ipst.history.remove(&key)))
}

#[derive(Embed)]
#[folder = "../target/frontend"]
struct FrontendAssets;

// Catch-all route to serve index.html for Vue routes
#[get("/<path..>")]
async fn serve_assets(path: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = path.display().to_string();

    // if the asset exist we serve it
    if let Some(asset) = FrontendAssets::get(&filename) {
        let content_type = path
            .extension()
            .and_then(OsStr::to_str)
            .and_then(ContentType::from_extension)
            .unwrap_or(ContentType::Bytes);
        Some((content_type, asset.data))
    } else {
        // if the asset doesn't exist we serve index.html
        // we delegate page routing to Vue
        let index = FrontendAssets::get("index.html")?;
        Some((ContentType::HTML, index.data))
    }
}

#[get("/openapi/json")]
async fn openapi() -> ApiResult<utoipa::openapi::OpenApi> {
    Ok(ApiData::Some(ApiDoc::openapi()))
}

#[derive(OpenApi)]
#[openapi(
    components(schemas(DataKind, SearchOrder)),
    paths(ip_new, ip_add_entry, ip_search_entry, ip_update_entry, ip_del_entry,)
)]
struct ApiDoc;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = connect_to_redis()?;

    rocket::build()
        .mount("/", routes![serve_assets])
        .mount(
            API_MOUNTPOINT,
            routes![
                openapi,
                ip_new,
                ip_add_entry,
                ip_search_entry,
                ip_update_entry,
                ip_del_entry,
            ],
        )
        .manage(Arc::new(Mutex::new(db)))
        .launch()
        .await?;
    Ok(())
}
