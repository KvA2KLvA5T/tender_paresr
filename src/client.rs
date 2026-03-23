use std::ops::RangeInclusive;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::time::Duration;
use std::sync::Arc;

use reqwest::{Client, RequestBuilder};
use reqwest::redirect::Policy;
use reqwest::cookie::Jar;

use scraper::{Html, Selector};

use super::error::Error;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

trait InitDefaultHeaders {
    fn default_headers(self) -> RequestBuilder;
}
impl InitDefaultHeaders for RequestBuilder {
    fn default_headers(self) -> RequestBuilder {
        self.header("authority", "rostender.info")
            .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
            .header("accept-language", "ru-RU,ru;q=0.9")
            .header("cache-control", "max-age=0")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("origin", "https://rostender.info")
            .header("referer", "https://rostender.info/extsearch")
            .header("sec-ch-ua", "\"Google Chrome\";v=\"143\", \"Chromium\";v=\"143\", \"Not A(Brand\";v=\"24\"")
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", "\"Windows\"")
            .header("sec-fetch-dest", "document")
            .header("sec-fetch-mode", "navigate")
            .header("sec-fetch-site", "same-origin")
            .header("upgrade-insecure-requests", "1")
    }
}


#[derive(Debug, Clone)]
pub struct ParserClient { 
    client: Client,
    url: Box<str>,
    pages_range: RangeInclusive<u16>,
    keyword: Arc<str>,
    stop_flag: StopFlag,
}
impl ParserClient {
    pub async fn new(keyword: &str, pages_limit: u16) -> Result<Self, Error> {
        trait InitFormData {
            fn init_form_data(self, keyword: &str) -> RequestBuilder;
        }
        impl InitFormData for RequestBuilder {
            fn init_form_data(self, keyword: &str) -> RequestBuilder {
                let form = [
                    ("path", "/extsearch"),
                    ("mode", "simple"),
                    ("default_search", "0"),
                    ("with_fo", "0"),
                    ("raw", ""),
                    ("open", "1"),
                    ("open_data", "1"),
                    ("fingerprint", ""),
                    ("geo", ""),
                    ("branch", ""),
                    ("keywords", keyword),
                ];
                self.form(&form)
            }
        }

        pub async fn pages_range(client: &Client, url: &str, pages_limit: u16) -> Result<RangeInclusive<u16>, Error> {
            let response = client.get(url)
                .default_headers()
                .send()
                .await?
                .error_for_status()?;
            
            let content = response.text().await?;
            let html = Html::parse_document(&content);
            let selector = Selector::parse("#paginationForm > div > input.form-control.set-page-form__input").unwrap();
            match html.select(&selector).next() {
                Some(el) => {
                    let start = el.attr("min")
                        .ok_or(Error::StrErr("Attribute \"min\" not found"))?
                        .parse::<u16>()
                        .map_err(|e| Error::ParseIntErr(e))?;
                    let mut end = el.attr("max")
                        .ok_or(Error::StrErr("Attribute \"max\" not found"))?
                        .parse::<u16>()
                        .map_err(|e| Error::ParseIntErr(e))?;
                    if end > pages_limit {
                        end = pages_limit
                    }
                    Ok(start..=end)
                },
                None => Ok(1..=1)
            }
        }
        
        let cookie_jar = Arc::new(Jar::default());
        
        let client = Client::builder()
            .cookie_provider(cookie_jar.clone())
            .redirect(Policy::none()) 
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(false) 
            .build()
            .unwrap();
        
        let initial_response = client
            .get("https://rostender.info/extsearch")
            .default_headers()
            .send()
            .await?;
        
        let status = initial_response.status().as_u16();
        if status != 200 {
            return Err(Error::StatusCodeErr(status));
        }
        
        let response = client
            .post("https://rostender.info/search/tenders") 
            .init_form_data(keyword)
            .default_headers()
            .send()
            .await
            .map_err(|e| Error::from(e))?;
        
        let url: Box<str> = response.headers()
            .get("location")
            .ok_or(Error::StrErr("NoUrlData"))?
            .to_str()
            .map_err(|_| Error::StrErr("failed to convert header to a str"))?
            .into();
        
        let pages_range = pages_range(&client, url.as_ref(), pages_limit).await?;
        let keyword = Arc::from(keyword);
        Ok(Self {client, url, pages_range, keyword, stop_flag: StopFlag::new()})
    }
    
    pub fn pages_range(&self) -> RangeInclusive<u16> {
        self.pages_range.clone()
    }
    pub async fn get_page_unchecked(&self, page: u16) -> Result<String, Error> {
        let url = format!("{}&page={}", self.url, page);
        let response = self.client
            .get(url)
            .default_headers()
            .send()
            .await?;
        Ok(response.text().await?)
    }
    pub async fn content_msg_unchecked(&self, page: u16) -> ContentMsg {
        let res = self.get_page_unchecked(page).await;
        ContentMsg {
            res,
            flag: self.stop_flag.clone(),
            page: page,
            keyword: Arc::clone(&self.keyword)
        }
    }
    pub fn is_stoped(&self) -> bool {
        self.stop_flag.is_stoped()
    }
}

#[derive(Debug)]
struct StopFlag(Arc<AtomicBool>);
impl StopFlag {
    fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    fn stop(&self) {
        self.0.store(true, Release);
    }
    fn is_stoped(&self) -> bool {
        self.0.load(Acquire)
    }
}
impl Clone for StopFlag {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

pub struct ContentMsg {
    pub res: Result<String, Error>,
    pub page: u16,
    pub keyword: Arc<str>, 
    flag: StopFlag,
}
impl ContentMsg {
    pub fn stop(&self) { self.flag.stop(); }
}
