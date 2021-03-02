#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

use anyhow::Result;
use argh::FromArgs;

use regex::Regex;

use std::{thread, time};

use flexi_logger::Logger;
use scraper::{ElementRef, Html, Selector};

use reqwest::redirect::Policy;

// Simple wrapper to get ALL descendent text from an element
fn get_text(node: ElementRef) -> String {
    node.text().collect::<Vec<&str>>().join(" ")
}

fn get_text_part(node: ElementRef, index: usize) -> Option<&str> {
    node.text().nth(index)
}

#[derive(FromArgs)]
/// Cobweb, a screen scraper for PrepMod (https://www.maimmunizations.org)
///
struct Args {
    /// show all clinics, even those with no availability
    #[argh(switch, short = 'a')]
    all: bool,

    /// start search from a date, for example, -f 2021-02-25
    #[argh(option, short = 'f')]
    from: Option<String>,

    /// filter results by name, for example, -n gillette
    #[argh(option, short = 'n')]
    name: Option<String>,

    /// wait in queue, instead of bailing
    #[argh(switch, short = 'w')]
    wait: bool,
}

/// We collect info we scrape in these records
struct ClinicInfo {
    pub name: String,
    pub date: Option<String>,
    pub availability: Option<String>,
    pub clinic_id: Option<String>,
    pub registration_url: Option<String>,
}

impl ClinicInfo {
    pub fn has_availability(&self) -> bool {
        match &self.availability {
            Some(a) => a != "0",
            None => false,
        }
    }

    pub fn report(&self, all: bool) {
        if self.has_availability() {
            if let Some(avail) = &self.availability {
                println!("{} has {} available", self.name_and_date(), avail);
            }

            if let Some(url) = &self.registration_url {
                println!("Register at {}", url);
            }

            println!(); // Extra newline to improve layout
        } else if all {
            println!("{} has no availability", self.name_and_date())
        }
    }

    pub fn name_and_date(&self) -> String {
        match &self.date {
            Some(d) => format!("{} on {}", self.name, d),
            None => self.name.clone(),
        }
    }

    pub fn new(
        name_and_date_str: &str,
        availability: Option<&str>,
        clinic_id: Option<&str>,
        registration_url: Option<&str>,
    ) -> ClinicInfo {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(.*) on (\d\d/\d\d/\d\d\d\d)$").unwrap();
        }

        // We look at the name string, apply a regex, to see if a date is embedded
        // if so, we split the name, and grab the date
        let name;
        let date;

        if let Some(caps) = RE.captures(name_and_date_str) {
            // We have name and date
            name = caps[1].to_string();
            date = Some(caps[2].to_string());
        } else {
            name = name_and_date_str.to_string();
            date = None;
        }

        ClinicInfo {
            name,
            date,
            availability: availability.map(|s| s.to_string()),
            clinic_id: clinic_id.map(|s| s.to_string()),
            registration_url: registration_url.map(|s| s.to_string()),
        }
    }
}

fn main() -> Result<()> {
    Logger::with_env_or_str("info").start()?;

    trace!("Started");

    let args: Args = argh::from_env();

    // Yes, we REALLY should validate this data,
    // Add some of the available query parameters

    // date such as 2021-02-25
    let date = args.from.unwrap_or_default();

    // clinic name such as gillette
    let venue_name = args.name.unwrap_or_default();

    let client = reqwest::blocking::Client::builder()
        .redirect(Policy::none())
        .build()?;

    let base_url = "https://www.maimmunizations.org";

    println!("Searching {}", base_url);
    println!(); // Extra newline to improve layout

    let mut page_number = 0;

    let mut infos: Vec<ClinicInfo> = Vec::new();

    loop {
        page_number += 1;

        let page_number_string = page_number.to_string();

        trace!("Fetching page {}", page_number_string);

        // Our main starting in point is the search page
        let search_url = format!("{}/clinic/search", base_url);

        let res = client
            .get(&search_url)
            .query(&[
                ("location", ""),
                ("search_radius", "All"),
                ("q[venue_search_name_or_venue_name_i_cont]", &venue_name),
                ("q[clinic_date_gteq]", &date),
                ("q[vaccinations_name_i_cont]", ""),
                ("commit", "Search"),
                ("page", &page_number_string),
            ])
            .send()?;

        let status = res.status();

        trace!("Response status {}", status);

        if !status.is_success() {
            if !status.is_redirection() {
                println!(
                    "Page {} fetch failed with unexpected status {}",
                    page_number_string, status
                );
            }

            break;
        }

        let raw = res.text()?;

        let document = Html::parse_document(&raw);

        let title_selector = Selector::parse("head > title").unwrap();

        let summary_selector =
            Selector::parse("#wrapper > main > div > section > section:nth-child(2) > h2").unwrap();

        // let mut title = None;
        if let Some(title_elem) = document.select(&title_selector).next() {
            if let Some(text) = title_elem.text().next() {
                let title = text.trim();

                trace!("Title is {}", title);

                if title == "Commonwealth of Massachusetts Virtual Waiting Room" {
                    if let Some(summary_elem) = document.select(&summary_selector).next() {
                        if let Some(text) = summary_elem.text().next() {
                            println!("{}", text.trim());
                        }
                    }

                    if args.wait {
                        trace!("Waiting before refresh");
                        thread::sleep(time::Duration::from_secs(10));
                        continue;
                    } else {
                        println!("Bailing cus we hit the waiting room!");
                        break;
                    }
                }
            }
        }

        let clinics_selector = Selector::parse(
        "body > div.main-container > div.mt-24.border-t.border-gray-200 > div.md\\:flex > div.md\\:flex-shrink",
        )
        .unwrap();

        // First thing in there
        let name_selector = Selector::parse("p").unwrap();
        let avail_selector = Selector::parse("p > strong").unwrap();

        // body > div.main-container > div.mt-24.border-t.border-gray-200 > div:nth-child(2) > div.md\:flex-shrink.text-gray-800 > p.my-3.flex > a
        let schedule_selector = Selector::parse("p > a").unwrap();

        for element in document.select(&clinics_selector) {
            if let Some(clinic) = scrape_clinic(
                element,
                base_url,
                &name_selector,
                &avail_selector,
                &schedule_selector,
            ) {
                infos.push(clinic);
            }
        }
    }

    let clinics = infos.len();

    // Report on results
    for clinic in &infos {
        clinic.report(args.all);
    }

    let clinics_with_availability = infos.iter().filter(|c| c.has_availability()).count();

    // Statistics
    println!(
        "Found {} clinics, {} with availability (fetched {} pages)",
        clinics,
        clinics_with_availability,
        page_number - 1
    );

    Ok(())
}

/// Given a DOM element try to scrape the information we need for a clinic
fn scrape_clinic(
    element: ElementRef,
    base_url: &str,
    name_selector: &Selector,
    avail_selector: &Selector,
    schedule_selector: &Selector,
) -> Option<ClinicInfo> {
    // We have to have at least a name...

    // The select is too generous, so use .next() to get just the first
    if let Some(name) = element.select(name_selector).next() {
        if let Some(text) = name.text().next() {
            let name = text.trim();

            // without a name, we cannot make use of this, but report this
            // as likely the page has changed in a way we do not expect
            if name.is_empty() {
                error!("Should have got a name for a clinic!");
                return None;
            }

            let mut availability: Option<&str> = None;

            // Get the availabilities, our selector returns ALL property labels
            // So we match text in code, then look to the parent for the value
            for label in element.select(&avail_selector) {
                //
                let text = get_text(label);
                if text.starts_with("Available Appointments") {
                    let parent = ElementRef::wrap(label.parent().unwrap()).unwrap();

                    if let Some(a) = get_text_part(parent, 1) {
                        availability = Some(a.trim())
                    }
                }
            }

            let mut registration_url: Option<String> = None;

            for schedule_link in element.select(&schedule_selector) {
                let href = schedule_link.value().attr("href");

                if let Some(href) = href {
                    let label = get_text(schedule_link);

                    trace!(">>>>> {} {}{}", label.trim(), base_url, href);

                    registration_url = Some(format!("{}{}", base_url, href));
                }
            }

            return Some(ClinicInfo::new(
                name,
                availability,
                None,
                registration_url.as_deref(),
            ));
        }
    }

    None
}
