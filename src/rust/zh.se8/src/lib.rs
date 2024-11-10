#![no_std]
extern crate alloc;

use aidoku::{
	error::Result,
	helpers::uri::encode_uri,
	prelude::*,
	std::{
		net::{HttpMethod, Request},
		String, Vec,
	},
	Chapter, Filter, FilterType, Manga, MangaContentRating, MangaPageResult, MangaStatus,
	MangaViewer, Page,
};
use alloc::string::ToString;

const WWW_URL: &str = "https://se8.us/index.php";

const FILTER_TAG: [&str; 22] = [
	"", "61", "63", "62", "64", "11", "15", "17", "29", "31", "67", "68", "69", "75", "78", "84",
	"86", "87", "91", "98", "106", "114",
];
const FILTER_FINISH: [&str; 3] = ["", "1", "2"];
const FILTER_ORDER: [&str; 2] = ["hits", "addtime"];

#[get_manga_list]
fn get_manga_list(filters: Vec<Filter>, page: i32) -> Result<MangaPageResult> {
	let mut query = String::new();
	let mut tag = String::new();
	let mut finish = String::new();
	let mut order = String::from("hits");

	for filter in filters {
		match filter.kind {
			FilterType::Title => {
				query = filter.value.as_string()?.read();
			}
			FilterType::Select => {
				let index = filter.value.as_int()? as usize;
				match filter.name.as_str() {
					"标签" => {
						tag = FILTER_TAG[index].to_string();
					}
					"进度" => {
						finish = FILTER_FINISH[index].to_string();
					}
					_ => continue,
				}
			}
			FilterType::Sort => {
				let value = match filter.value.as_object() {
					Ok(value) => value,
					Err(_) => continue,
				};
				let index = value.get("index").as_int()? as usize;
				order = FILTER_ORDER[index].to_string();
			}
			_ => continue,
		}
	}

	let url = if query.is_empty() {
		let mut url = format!("{}/category", WWW_URL);

		if !tag.is_empty() {
			url.push_str(&format!("/tags/{}", tag));
		}

		if !finish.is_empty() {
			url.push_str(&format!("/finish/{}", finish));
		}

		format!("{}/order/{}/page/{}", url, order, page)
	} else {
		format!("{}/search/{}/{}", WWW_URL, encode_uri(query), page)
	};
	let html = Request::new(url, HttpMethod::Get).html()?;
	let has_more = true;
	let mut mangas: Vec<Manga> = Vec::new();

	for item in html.select(".comic-item").array() {
		let item = match item.as_node() {
			Ok(node) => node,
			Err(_) => continue,
		};
		let id = item
			.select("a")
			.attr("href")
			.read()
			.split("/")
			.map(|a| a.to_string())
			.collect::<Vec<String>>()
			.pop()
			.unwrap();
		let cover = item.select("a>img").attr("data-original").read();
		let title = item.select("p:nth-child(2)>a").text().read();
		mangas.push(Manga {
			id,
			cover,
			title,
			..Default::default()
		});
	}

	Ok(MangaPageResult {
		manga: mangas,
		has_more,
	})
}

#[get_manga_details]
fn get_manga_details(id: String) -> Result<Manga> {
	let url = format!("{}/comic/{}", WWW_URL, id.clone());
	let html = Request::new(url.clone(), HttpMethod::Get).html()?;
	let id = html.select(".j-user-collect").attr("data-id").read();
	let cover = html.select(".de-info__cover>img").attr("src").read();
	let title = html.select(".j-comic-title").text().read();
	let author = html
		.select(".comic-author>.name>a")
		.text()
		.read()
		.replace("&amp", "&")
		.split("&")
		.filter(|a| !a.trim().is_empty())
		.map(|a| a.to_string())
		.collect::<Vec<String>>()
		.join(", ");
	let artist = String::new();
	let description = html
		.select(".comic-intro>.intro")
		.text()
		.read()
		.trim()
		.replace("&hellip", "…");
	let categories = html
		.select(".comic-status>span:nth-child(1)>b>a")
		.array()
		.map(|a| a.as_node().unwrap().text().read().trim().to_string())
		.filter(|a| !a.is_empty())
		.collect::<Vec<String>>();
	let status = MangaStatus::Ongoing;
	let nsfw = MangaContentRating::Nsfw;
	let viewer = MangaViewer::Scroll;

	Ok(Manga {
		id,
		cover,
		title,
		author,
		artist,
		description,
		url,
		categories,
		status,
		nsfw,
		viewer,
	})
}

#[get_chapter_list]
fn get_chapter_list(id: String) -> Result<Vec<Chapter>> {
	let url = format!("{}/api/comic/chapter?mid={}", WWW_URL, id.clone());
	let json = Request::new(url.clone(), HttpMethod::Get).json()?;
	let data = json.as_object()?;
	let list = data.get("data").as_array()?;
	let mut chapters: Vec<Chapter> = Vec::new();

	for (index, item) in list.enumerate() {
		let item = match item.as_object() {
			Ok(item) => item,
			Err(_) => continue,
		};
		let id = item.get("id").as_string()?.read();
		let title = item
			.get("name")
			.as_string()?
			.read()
			.trim()
			.replace("&lt;", "<")
			.replace("&gt;", ">")
			.replace("&#40;", "(")
			.replace("&#41;", ")")
			.replace("&ldquo;", "“")
			.replace("&rdquo;", "”")
			.replace("&hellip;", "…")
			.replace("&hearts;", "♥");
		let chapter = (index + 1) as f32;
		let url = item.get("link").as_string()?.read();
		chapters.push(Chapter {
			id,
			title,
			chapter,
			url,
			..Default::default()
		});
	}
	chapters.reverse();

	Ok(chapters)
}

#[get_page_list]
fn get_page_list(_: String, chapter_id: String) -> Result<Vec<Page>> {
	let url = format!("{}/chapter/{}", WWW_URL, chapter_id.clone());
	let html = Request::new(url.clone(), HttpMethod::Get).html()?;
	let mut pages: Vec<Page> = Vec::new();

	for (index, item) in html.select("div[id^='pic']>img").array().enumerate() {
		let item = match item.as_node() {
			Ok(node) => node,
			Err(_) => continue,
		};
		let index = index as i32;
		let url = item.attr("data-original").read().trim().to_string();
		pages.push(Page {
			index,
			url,
			..Default::default()
		})
	}

	Ok(pages)
}
