use std::fmt::Debug;
use std::sync::LazyLock;
use std::ptr::NonNull;

use scraper::{ElementRef, Html, Selector};
use scraper::html::Select;

use chrono::NaiveDate;

use serde::{Serialize, Deserialize};

use crate::error::Error;

static SELECTORS: LazyLock<Selectors> = LazyLock::new(|| {
    Selectors { 
        info_block_selector: Selector::parse("div.tender__info").unwrap(), 
        number_selector: Selector::parse("span.tender__number").unwrap(), 
        date_selector: Selector::parse("span.tender__date-start").unwrap(), 
        description_selector: Selector::parse("a").unwrap() }
});

struct Selectors {
    info_block_selector: Selector,
    number_selector: Selector,
    date_selector: Selector,
    description_selector: Selector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tender {
    pub id: usize,
    pub date: Option<NaiveDate>,
    pub description: Box<str>,
    pub url: Box<str>,
}

pub struct TendersIter {
    ptr: NonNull<Html>,
    iter: Select<'static, 'static>,
}
impl TendersIter {
    pub fn new(content: &str) -> Self {
        let html = Box::new(Html::parse_document(content));
        let leaked: &'static mut Html = Box::leak(html);
        let ptr = NonNull::from_mut(leaked);
        let iter = leaked.select(&SELECTORS.info_block_selector);
        Self {
            ptr,
            iter,
        }
    }
}
impl Drop for TendersIter {
    fn drop(&mut self) {
        let _ = unsafe { Box::from_raw(self.ptr.as_ptr()) };
    }
}
impl Iterator for TendersIter {
    type Item = Result<Tender, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        fn parse_number(number: &str) -> Result<usize, Error> {
            let start = number.find('№');
            let end = number.rfind(|ch: char| ch.is_numeric());
            match (start, end) {
                (Some(start), Some(end)) => {
                    (&number[start+3..=end]).parse::<usize>().map_err(|e| Error::ParseIntErr(e))
                },
                _ => Err(Error::StrErr("ParseNumberError")),
            }
        }
        fn parse_date(date: &str) -> Result<NaiveDate, Error> {
            let start = date.find(|ch: char| ch.is_numeric());
            let end = date.rfind(|ch: char| ch.is_numeric());
            match (start, end) {
                (Some(start), Some(end)) => {
                    let date_str = &date[start..=end];
                    NaiveDate::parse_from_str(date_str, "%d.%m.%y").map_err(|_| Error::StrErr("ParseDateError"))
                },
                _ => Err(Error::StrErr("ParseDateError"))
            }
        }
        fn parse_element(el: ElementRef) -> Result<Tender, Error> {
            let id = el.select(&SELECTORS.number_selector)
                .next()
                .map(|elem| elem.text().next())
                .flatten()
                .ok_or(Error::StrErr("NumberElementNoFound"))
                .map(|num| parse_number(num))??;
            
            let date = el.select(&SELECTORS.date_selector)
                .next()
                .map(|elem| elem.text().next())
                .flatten()
                .map_or(Ok(None), |date_str| { 
                    parse_date(date_str).map(|date| Some(date))
                })?;
            
            let (url, description) = el.select(&SELECTORS.description_selector)
                .next()
                .map(|el| {
                    match (el.attr("href"), el.attr("title")) {
                        (Some(url), Some(description)) => Ok((url, description)),
                        _ => Err(Error::StrErr("DescriptionElementNoFound")),
                    }
                })
                .ok_or(Error::StrErr("DescriptionElementNoFound"))??;
            let url = {
                let mut full_url = String::from("https://rostender.info");
                full_url.push_str(url);
                full_url.into_boxed_str()
            };
            let description: Box<str> = Box::from(description);
            Ok(Tender {id, date, url, description,})
            
        }
        let el = self.iter.next()?;
        Some(parse_element(el))
        
    }
}
impl Debug for TendersIter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TendersIter {{..}}")
    }
}
