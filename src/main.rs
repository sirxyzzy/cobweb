#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

use anyhow::Result;
use argh::FromArgs;

use regex::Regex;

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

    pub fn new(name_str: &str, availability: Option<&str>) -> ClinicInfo {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(.*) on (\d\d/\d\d/\d\d\d\d)$").unwrap();
        }

        let availability = availability.map(|s| s.to_string());

        match RE.captures(name_str) {
            Some(caps) => {
                // We have name and date
                let name = &caps[1];
                let date_str = &caps[2];

                ClinicInfo {
                    name: name.to_string(),
                    date: Some(date_str.to_string()),
                    availability,
                    clinic_id: None,
                    registration_url: None,
                }
            }
            None => {
                // Just a name (never seen?)
                ClinicInfo {
                    name: name_str.to_string(),
                    date: None,
                    availability,
                    clinic_id: None,
                    registration_url: None,
                }
            }
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

        if !res.status().is_success() {
            if !res.status().is_redirection() {
                println!(
                    "Page {} fetch failed with unexpected status {}",
                    page_number_string,
                    res.status()
                );
            }

            break;
        }

        let raw = res.text()?;

        let document = Html::parse_document(&raw);

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
            // println!("{:#?}", element.value());

            // Get the NAME
            let mut maybe_name: Option<&str> = None;
            // The select is too generous, so use .next() to get just the first
            if let Some(name) = element.select(&name_selector).next() {
                if let Some(text) = name.text().next() {
                    maybe_name = Some(text.trim());
                }
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

            let mut registration_url = None;

            for schedule_link in element.select(&schedule_selector) {
                let href = schedule_link.value().attr("href");

                if let Some(href) = href {
                    let label = get_text(schedule_link);

                    trace!(">>>>> {} {}{}", label.trim(), base_url, href);

                    registration_url = Some(format!("{}{}", base_url, href));
                }
            }

            if let Some(name) = maybe_name {
                // infos.push(ClinicInfo {
                //     name: name.to_string(),
                //     date: None,
                //     availability: availability.map(|s| s.to_string()),
                //     clinic_id: None,
                //     registration_url,
                // });
                infos.push(ClinicInfo::new(name, availability))
            }
        }
    }

    let mut clinics_with_availability = 0;
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
