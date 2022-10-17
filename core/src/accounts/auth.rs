//! This library is for logging into a minecraft account using the microsoft oauth2 device flow: https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-device-code
//! # Example
//! ```no_run
//! use {ms_auth_mc::*, reqwest::blocking::Client};
//!
//! let client = Client::new();
//! let device_code =
//!     DeviceCode::new("221e73fa-365e-4263-9e06-7a0a1f277960"/* You would ideally replace this with your own CID which you can get from creating an azure application*/, None, &client).unwrap();
//!
//! if let Some(inner) = &device_code.inner {
//!    println!("{}", inner.message);
//! }
//!
//! let auth = device_code.authenticate(&client).unwrap();
//! println!("{}", auth.token);
//! ```

const CACHE_FILE_NAME: &str = "auth.cache";

use {
    base64::{read::DecoderReader, write::EncoderWriter},
    std::{fs, io::Read, io::Write, path::Path, string::String},
};

use {
    anyhow::bail,
    byteorder::{ReadBytesExt, WriteBytesExt, LE},
    reqwest::{Client, StatusCode},
    serde::{Deserialize, Serialize},
    serde_json::json,
};

fn write_string_to(w: &mut impl Write, s: &String) -> anyhow::Result<()> {
    let len = s.len();
    w.write_u16::<LE>(len as u16)?;
    w.write_all(s.as_bytes())?;
    Ok(())
}

fn read_string_from(r: &mut impl Read) -> anyhow::Result<String> {
    let len = r.read_u16::<LE>()?;
    let mut buf = vec![0; len as usize];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf)?)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Auth {
    pub name: String,
    pub uuid: String,
    pub token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct McProfile {
    id: String,
    name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct McAuth {
    pub access_token: String,
    pub expires_in: i64,
    //#[serde(skip)]
    //pub expires_after: i64,
}

impl McAuth {
    async fn mc_profile(&self, client: &Client) -> anyhow::Result<McProfile> {
        let pr_resp = client
            .get("https://api.minecraftservices.com/minecraft/profile")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?
            .json()
            .await?;

        Ok(pr_resp)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XstsAuth {
    token: String,
    display_claims: DisplayClaims,
}

impl XstsAuth {
    async fn auth_mc(&self, client: &Client) -> anyhow::Result<McAuth> {
        let json = json!({
            "identityToken": format!("XBL3.0 x={};{}", self.display_claims.xui[0].uhs, self.token)
        });

        let mc_resp = client
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .header("Accept", "application/json")
            .json(&json)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        //mc_auth.expires_after = mc_auth.expires_in + chrono::Utc::now().timestamp();
        Ok(mc_resp)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XblAuth {
    token: String,
}

impl XblAuth {
    async fn auth_xsts(&self, client: &Client) -> anyhow::Result<XstsAuth> {
        let json = json!({
            "Properties": {
                "SandboxId":  "RETAIL",
                "UserTokens": [self.token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType":    "JWT",
        });

        let xsts_resp = client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .header("Content-Type", "application/json")
            .json(&json)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(xsts_resp)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MsAuthRefresh {
    expires_in: i64,
    access_token: String,
    refresh_token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MsAuth {
    expires_in: i64,
    access_token: String,
    refresh_token: String,
    #[serde(skip)]
    expires_after: i64,
}

impl MsAuth {
    /// Checks if the access token is still valid and refreshes it if it isn't.
    pub async fn refresh(&mut self, cid: &str, client: &Client) -> anyhow::Result<bool> {
        if self.expires_after <= chrono::Utc::now().timestamp() {
            let resp: MsAuthRefresh = client
                .post("https://login.live.com/oauth20_token.srf")
                .form(&[
                    ("client_id", cid),
                    ("refresh_token", &self.refresh_token),
                    ("grant_type", "refresh_token"),
                    (
                        "redirect_uri",
                        "https://login.microsoftonline.com/common/oauth2/nativeclient",
                    ),
                ])
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            self.access_token = resp.access_token;
            self.refresh_token = resp.refresh_token;
            self.expires_after = resp.expires_in + chrono::Utc::now().timestamp();
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn auth_xbl(&self, client: &Client) -> anyhow::Result<XblAuth> {
        let json = json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName":   "user.auth.xboxlive.com",
                "RpsTicket":  &(String::from("d=") + &self.access_token) as &str,
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType":    "JWT",
        });

        let xbl_resp = client
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .header("Accept", "application/json")
            .json(&json)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(xbl_resp)
    }

    pub fn write_to(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let mut w = EncoderWriter::new(w, base64::STANDARD);
        let mut buf = Vec::new();
        buf.write_i64::<LE>(self.expires_after)?;
        write_string_to(&mut buf, &self.access_token)?;
        write_string_to(&mut buf, &self.refresh_token)?;
        let len = buf.len();
        w.write_u16::<LE>(len as u16)?;
        w.write_all(&buf)?;
        Ok(())
    }

    pub fn read_from(r: &mut impl Read) -> anyhow::Result<MsAuth> {
        let mut r = DecoderReader::new(r, base64::STANDARD);
        let len = r.read_u16::<LE>()? as usize;
        let mut buf = vec![0; len];
        r.read_exact(&mut buf)?;
        let mut buf = buf.as_slice();
        let expires_after = buf.read_i64::<LE>()?;
        let access_token = read_string_from(&mut buf)?;
        let refresh_token = read_string_from(&mut buf)?;
        Ok(MsAuth {
            expires_in: 0,
            access_token,
            refresh_token,
            expires_after,
        })
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MsAuthError {
    error: String,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct DeviceCode {
    pub inner: Option<DeviceCodeInner>,
    cid: String,
    cache: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceCodeInner {
    pub user_code: String,
    device_code: String,
    pub verification_uri: String,
    expires_in: i64,
    interval: u64,
    pub message: String,
}

impl DeviceCode {
    /// Entry point of the auth flow.
    /// It's up to you how you show the user the user code and the link
    /// Only show the user code and the link when cached is false because they'll be empty if not.
    pub async fn new(cid: &str, cache_file: Option<&str>, client: &Client) -> anyhow::Result<Self> {
        let (path, name) = match cache_file {
            Some(file) => (Path::new(file), file),
            None => (Path::new(CACHE_FILE_NAME), CACHE_FILE_NAME),
        };

        let device_code: DeviceCode;
        let device_code_inner: Option<DeviceCodeInner>;
        if !path.is_file() {
            let device_resp = client
                .get("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
                .query(&[
                    ("client_id", cid),
                    ("scope", "XboxLive.signin offline_access"),
                ])
                .header("content-length", "0")
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            device_code_inner = Some(device_resp);
        } else {
            device_code_inner = None;
        }
        device_code = DeviceCode {
            inner: device_code_inner,
            cid: String::from(cid),
            cache: String::from(name),
        };
        Ok(device_code)
    }

    async fn auth_ms(&self, client: &Client) -> anyhow::Result<Option<MsAuth>> {
        match &self.inner {
            Some(inner) => loop {
                std::thread::sleep(std::time::Duration::from_secs(inner.interval + 1));

                let code_resp = client
                    .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
                    .form(&[
                        ("client_id", &self.cid as &str),
                        ("scope", "XboxLive.signin offline_access"),
                        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                        ("device_code", &inner.device_code),
                    ])
                    .send()
                    .await?;

                match code_resp.status() {
                    StatusCode::BAD_REQUEST => {
                        let ms_auth: MsAuthError = code_resp.json().await?;
                        match &ms_auth.error as &str {
                            "authorization_pending" => continue,
                            _ => {
                                bail!("{}", ms_auth.error)
                            }
                        }
                    }
                    StatusCode::OK => {
                        let mut ms_auth: MsAuth = code_resp.json().await?;
                        ms_auth.expires_after = ms_auth.expires_in + chrono::Utc::now().timestamp();
                        return Ok(Some(ms_auth));
                    }
                    _ => {
                        return Err(anyhow::Error::msg(format!(
                            "unexpected response code: {}",
                            code_resp.status().as_str()
                        )))
                    }
                }
            },
            None => Ok(None),
        }
    }

    /// Call this method after creating the device code and having shown the user the code (but only if DeviceCode.cached is false)
    /// It might block for a while if the access token hasn't been cached yet.
    pub async fn authenticate(&self, client: &Client) -> anyhow::Result<Auth> {
        let path: &Path = Path::new(&self.cache);
        let msa = match self.inner {
            Some(_) => {
                let msa = self.auth_ms(client).await?.unwrap();
                msa.write_to(&mut fs::File::create(path)?)?;
                msa
            }
            None => {
                let mut msa: MsAuth = MsAuth::read_from(&mut fs::File::open(path)?)?;
                if msa.refresh(&self.cid, client).await? {
                    msa.write_to(&mut fs::File::create(path)?)?;
                }
                msa
            }
        };
        let mca = msa
            .auth_xbl(client)
            .await?
            .auth_xsts(client)
            .await?
            .auth_mc(client)
            .await?;

        let profile = mca.mc_profile(client).await?;

        let auth = Auth {
            name: profile.name,
            uuid: profile.id,
            token: mca.access_token,
        };
        Ok(auth)
    }
}
