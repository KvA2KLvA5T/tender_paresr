use std::collections::HashSet;
use std::fmt::Write;
use std::thread::JoinHandle;
use std::sync::mpsc;

use chrono::{Datelike, Timelike};

use crate::client::{ParserClient, ContentMsg};
use crate::page_parser::Tender;
use crate::error::Error;
use crate::page_parser::TendersIter;
use crate::storage::TendersStorage;


#[derive(Debug)]
pub struct ParsingResult {
    pub tenders: Vec<Tender>,
    pub report: String
}

impl ParsingResult {
    pub fn render(&self) -> String {
        const HEAD: &str = include_str!("./head.html");
        let capacity = HEAD.len() + 500*self.tenders.len();
        let mut s = String::with_capacity(capacity);
        s.push_str(HEAD);
        s.push_str("<body>");
        for tender in self.tenders.iter() {
            let _ = write!(
                &mut s, 
                "<div class=\"block\"><div class=\"row\"><div class=\"number\">Тендер №{}</div>", 
                tender.id
            );
            if let Some(date) = tender.date {
                let _ = write!(&mut s, "<div class=\"date\">от {:0>2}.{:0>2}.{}</div>", date.day(), date.month(), date.year());
            }
            let _ = write!(
                &mut s, 
                "</div><div class=\"description\"><a href=\"{}\" target=\"_blank\">{}</a></div></div>\n",
                tender.url, tender.description
            );
        }
        s.push_str("</body></html>");
        s
    }
    pub fn save_report(&self) {
        let _ = std::fs::create_dir("./log");
        let date = chrono::Local::now().naive_local();
        let path = format!("./log/{}{:02}{:02}{:02}{:02}{:02}.txt",
            date.year(), date.month(), date.day(),
            date.hour(), date.minute(), date.second()
        );
        let _ = std::fs::write(path, &self.report);
    }
}

pub struct Workers {
    parser_worker: JoinHandle<Result<ParsingResult, Error>>,
    requests_worker: JoinHandle<Result<(), Error>>,
}
impl Workers {
    pub fn new<I, Item>(keywords: I, pages_limit: u16) -> Self 
    where
        I: IntoIterator<Item = Item> + Send + 'static,
        Item: AsRef<str>
    {
        fn create_parser_worker(receiver: mpsc::Receiver<ContentMsg>) -> JoinHandle<Result<ParsingResult, Error>> {
            std::thread::spawn(move || {
                let storage_init_result = TendersStorage::init();
                let mut storage = match storage_init_result {
                    Ok(storage) => storage,
                    Err(e) => return Err(e),
                };
                let mut report = String::new();
                let mut sub_report = String::new();
                let mut tenders = Vec::new();
                let mut tender_ids = HashSet::new();
                loop {
                    match receiver.recv() {
                        Ok(msg) => {
                            match msg.res.as_ref() {
                                Ok(content) => {
                                    let mut count = 0u16;
                                    let mut contains_count = 0u16;
                                    for (idx, tender) in TendersIter::new(content).enumerate() {
                                        let idx = idx + 1;
                                        match tender {
                                            Ok(tender) => {
                                                if tender_ids.contains(&tender.id) {
                                                    let _ = write!(&mut sub_report, "\t{idx:3}. [ PASS ] Tender number {} already exists in list.\n", tender.id);
                                                } else {
                                                    count += 1;
                                                    tender_ids.insert(tender.id);
                                                    if storage.contains(&tender) { 
                                                        let _ = write!(&mut sub_report, "\t{idx:3}. [ PASS ] Tender number {} was added in another session.\n", tender.id);
                                                        contains_count += 1; 
                                                    }
                                                    else { 
                                                        let _ = write!(&mut sub_report, "\t{idx:3}. [  OK  ] Tender number {} has been added to the list.\n", tender.id);
                                                        storage.push(tender.clone());
                                                        tenders.push(tender); 
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                let _ = write!(&mut sub_report, "\t{idx:3}. [ ERROR ] '{}'\n", e);
                                            },
                                        }
                                    }
                                    let _ = write!(&mut report, "{count} tenders were found on page {} for the query '{}'\n{}", msg.page, msg.keyword,  &sub_report);

                                    sub_report.clear();
                                    if count == contains_count { msg.stop() }
                                },
                                
                                Err(e) => {
                                    let _ = write!(&mut report, "An error occurred on page {} with the query '{}': '{e}'\n", msg.page, msg.keyword);
                                    
                                },
                            }
                        },
                        Err(_) => {break Ok(ParsingResult{tenders, report});},
                    }
                }
            })
        }
        
        pub async fn init_clients<I, Item>(keywords: I, pages_limit: u16) -> Result<Vec<ParserClient>, Error> 
        where
            I: IntoIterator<Item = Item>,
            Item: AsRef<str>
        {
            let vec_of_futures = keywords.into_iter()
                .map(|keyword| {
                    async move {
                        ParserClient::new(keyword.as_ref(), pages_limit).await
                    }
                })
                .collect::<Vec<_>>();
            
            futures::future::join_all(vec_of_futures)
                .await
                .into_iter()
                .collect()
        }
        
        pub async fn run_requests(clients: Vec<ParserClient>, sender: mpsc::Sender<ContentMsg>) {
            let joins = clients.into_iter().map(|client| {
                let sender_clone = sender.clone();
                tokio::spawn(async move {
                    let sender = sender_clone;
                    for page in client.pages_range() {
                        if client.is_stoped() { break; }
                        let content = client.content_msg_unchecked(page).await;
                        if let Err(_) = sender.send(content) {break;}
                    }
                })
            }).collect::<Vec<_>>();
            for j in joins.into_iter() {
                let _ = j.await;
            }
            drop(sender);
        }
        
        pub fn create_requests_worker<I, Item>(sender: mpsc::Sender<ContentMsg>, keywords: I, pages_limit: u16) -> JoinHandle<Result<(), Error>>
        where
            I: IntoIterator<Item = Item> + Send + 'static,
            Item: AsRef<str>
        {
            std::thread::spawn(move || {
                let run_time = tokio::runtime::Runtime::new().unwrap();
                run_time.block_on(async move {
                    let clients_res = init_clients(keywords, pages_limit)
                        .await;
                    
                    match clients_res {
                        Ok(clients) => { 
                            run_requests(clients, sender).await; 
                        }
                        Err(e) => { return Err(e); },
                    }
                    Ok(())
                })
            })
        }
        

        let (sender, receiver) = mpsc::channel::<ContentMsg>();
        
        let parser_worker = create_parser_worker(receiver);
        let requests_worker = create_requests_worker(sender, keywords, pages_limit);
        Self {parser_worker, requests_worker}
    }
    
    
    pub fn join(self) -> Result<Result<ParsingResult, Error>, Error> {
        let requests_worker_res = self.requests_worker.join().map_err(|_| Error::StrErr("Panic in requests worker thread"))?;
        let parser_worker_res = self.parser_worker.join().map_err(|_| Error::StrErr("Panic in parser worker thread"))?;
        if let Err(e) = requests_worker_res { return Ok(Err(e)) }
        Ok(parser_worker_res)
    }
}