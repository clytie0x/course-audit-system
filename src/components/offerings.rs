//! Course offerings from SIS (static JSON under `offerings/`), with search, an
//! "outstanding required courses only" filter, prerequisite badges, and a
//! personal timetable that flags class-time and exam clashes.

use audit_core::auditor::unmet_prerequisites;
use audit_core::data::curriculum;
use audit_core::offerings::{
    outstanding_codes, search, sections_clash, Offering, OfferingsIndex, Section, TermOfferings,
};
use leptos::prelude::*;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::app::AppState;
use crate::components::ui::{CARD, SECTION_TITLE, TAG, TAG_WARN};

/// Results shown at once; refine the search to see more.
const MAX_RESULTS: usize = 40;
const DAYS_TH: [&str; 8] = ["", "จ.", "อ.", "พ.", "พฤ.", "ศ.", "ส.", "อา."];
const DAYS_EN: [&str; 8] = ["", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

pub(crate) async fn fetch_json<T: DeserializeOwned>(url: String) -> Option<T> {
    let window = web_sys::window()?;
    let resp: web_sys::Response = JsFuture::from(window.fetch_with_str(&url))
        .await
        .ok()?
        .dyn_into()
        .ok()?;
    if !resp.ok() {
        return None;
    }
    let text = JsFuture::from(resp.text().ok()?).await.ok()?.as_string()?;
    serde_json::from_str(&text).ok()
}

/// A section picked into the personal timetable.
#[derive(Clone, PartialEq)]
struct Pick {
    code: String,
    name: String,
    section: Section,
}

impl Pick {
    fn label(&self) -> String {
        format!("{} ({})", self.code, self.section.section)
    }
}

#[component]
pub fn OfferingsPanel() -> impl IntoView {
    let AppState {
        is_thai,
        parsed_courses,
        planned_courses,
        curriculum_id,
        ..
    } = expect_context::<AppState>();

    let index = LocalResource::new(|| fetch_json::<OfferingsIndex>("offerings/index.json".into()));
    let (term, set_term) = signal(None::<String>);
    let selected_term = move || {
        term.get()
            .or_else(|| index.get().flatten().and_then(|i| i.terms.first().cloned()))
    };
    let offerings = LocalResource::new(move || {
        let key = selected_term();
        async move {
            match key {
                Some(k) => fetch_json::<TermOfferings>(format!("offerings/{k}.json")).await,
                None => None,
            }
        }
    });
    let (query, set_query) = signal(String::new());
    let (only_outstanding, set_only_outstanding) = signal(false);
    let (picks, set_picks) = signal(Vec::<Pick>::new());

    // Transcript + plan, used for prerequisite and "outstanding" checks.
    let attempts = move || {
        let mut all = parsed_courses.get();
        all.extend(planned_courses.get());
        all
    };

    let results = move || {
        let Some(data) = offerings.get().flatten() else {
            return Vec::new();
        };
        let cur = curriculum(&curriculum_id.get());
        // Only what the transcript still lacks; planned courses are not taken yet.
        let outstanding = outstanding_codes(&parsed_courses.get(), cur);
        search(&data.courses, &query.get())
            .into_iter()
            .filter(|c| !only_outstanding.get() || outstanding.contains(&c.code))
            .take(MAX_RESULTS)
            .cloned()
            .collect::<Vec<Offering>>()
    };

    let t = move |th: &'static str, en: &'static str| if is_thai.get() { th } else { en };

    view! {
        <section class=format!("{CARD} overflow-hidden print:hidden")>
            <div class="flex flex-wrap items-center gap-x-2.5 gap-y-1 px-4 sm:px-[18px] pt-4 pb-1.5">
                <h2 class=SECTION_TITLE>{move || t("วิชาที่เปิดสอน", "Course offerings")}</h2>
                <span class="flex-1"></span>
                <span class="text-[12.5px] text-zinc-500">
                    {move || offerings.get().flatten().map(|d| format!("SIS · {} · {}", d.campus, d.scraped_at))}
                </span>
            </div>
            <div class="px-4 sm:px-[18px] pb-4 pt-2 space-y-4">
                <div class="flex flex-wrap gap-2.5 items-center">
                    <select
                        class="h-10 rounded-[9px] border border-zinc-200 bg-white px-3 text-sm tabular-nums text-zinc-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                        aria-label=move || t("ภาคการศึกษา", "Term")
                        prop:value=move || selected_term().unwrap_or_default()
                        on:change=move |ev| set_term.set(Some(event_target_value(&ev)))
                    >
                        {move || index.get().flatten().map(|i| i.terms.into_iter().map(|k| {
                            let (year, sem) = k.split_once('-').unwrap_or((&k, ""));
                            let label = format!("{sem}/{year}");
                            view! { <option value=k.clone()>{label}</option> }
                        }).collect::<Vec<_>>())}
                    </select>
                    <input
                        type="search"
                        placeholder=move || t("ค้นหารหัสหรือชื่อวิชา", "Search code or name")
                        aria-label=move || t("ค้นหารายวิชาที่เปิดสอน", "Search offerings")
                        class="h-10 flex-1 min-w-[180px] rounded-[9px] border border-zinc-200 bg-white px-3 text-sm text-zinc-900 placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                        prop:value=move || query.get()
                        on:input=move |ev| set_query.set(event_target_value(&ev))
                    />
                    // Only meaningful with a transcript.
                    {move || (!parsed_courses.get().is_empty()).then(|| view! {
                        <label class="inline-flex items-center gap-2 text-[13.5px] text-zinc-600 cursor-pointer">
                            <input type="checkbox" class="w-4 h-4 rounded accent-zinc-900"
                                prop:checked=move || only_outstanding.get()
                                on:change=move |ev| set_only_outstanding.set(event_target_checked(&ev))
                            />
                            {move || t("เฉพาะวิชาบังคับที่ยังไม่ได้เรียน", "Required courses not yet taken")}
                        </label>
                    })}
                </div>

                // Personal timetable
                {move || {
                    let list = picks.get();
                    (!list.is_empty()).then(|| {
                        let clashes: Vec<String> = list.iter().enumerate().flat_map(|(i, a)| {
                            list.iter().skip(i + 1).filter_map(move |b| sections_clash(&a.section, &b.section).map(|c| {
                                let mut what = Vec::new();
                                if c.class_time { what.push(if is_thai.get_untracked() { "เวลาเรียน" } else { "class time" }); }
                                if c.midterm { what.push(if is_thai.get_untracked() { "สอบกลางภาค" } else { "midterm" }); }
                                if c.final_exam { what.push(if is_thai.get_untracked() { "สอบปลายภาค" } else { "final" }); }
                                format!("{} × {}: {}", a.label(), b.label(), what.join(", "))
                            }))
                        }).collect();
                        let mut slots: Vec<(u8, String, String)> = list.iter().flat_map(|p| p.section.meetings.iter().map(move |m| {
                            (m.day, format!("{}–{}", m.start, m.end), format!("{} {}", p.label(), m.room))
                        })).collect();
                        slots.sort();
                        view! {
                            <div class="rounded-xl border border-accent-200 bg-accent-50/60 p-4 space-y-2.5">
                                <div class="flex items-center justify-between">
                                    <p class="text-xs font-semibold text-accent-700">{move || t("ตารางเรียนของฉัน", "My timetable")}</p>
                                    <button class="text-2xs font-semibold text-zinc-500 hover:text-red-600" on:click=move |_| set_picks.set(Vec::new())>
                                        {move || t("ล้าง", "Clear")}
                                    </button>
                                </div>
                                <ul class="space-y-1">
                                    {slots.into_iter().map(|(day, time, what)| view! {
                                        <li class="text-[13px] font-mono text-zinc-700">
                                            <span class="inline-block w-10 text-zinc-500">{move || if is_thai.get() { DAYS_TH[day as usize % 8] } else { DAYS_EN[day as usize % 8] }}</span>
                                            <span class="inline-block w-28">{time}</span>
                                            {what}
                                        </li>
                                    }).collect::<Vec<_>>()}
                                </ul>
                                {(!clashes.is_empty()).then(|| view! {
                                    <ul class="space-y-1" role="alert">
                                        {clashes.into_iter().map(|c| view! {
                                            <li class="text-xs font-medium text-red-700">{format!("⚠ {c}")}</li>
                                        }).collect::<Vec<_>>()}
                                    </ul>
                                })}
                            </div>
                        }
                    })
                }}

                // Results
                {move || match offerings.get() {
                    None => view! { <p class="text-xs text-zinc-500">{move || t("กำลังโหลด…", "Loading…")}</p> }.into_any(),
                    Some(None) => view! { <p class="text-xs text-zinc-500">{move || t("ยังไม่มีข้อมูลรายวิชาที่เปิดสอน", "No offering data available")}</p> }.into_any(),
                    Some(Some(_)) => {
                        let cur = curriculum(&curriculum_id.get());
                        let done = attempts();
                        let rows = results();
                        let empty = rows.is_empty();
                        view! {
                            <div class="divide-y divide-zinc-200 rounded-[9px] border border-zinc-200">
                                {rows.into_iter().map(|c| {
                                    let unmet = unmet_prerequisites(&c.code, &done, cur);
                                    let code = c.code.clone();
                                    let name = c.name_th.clone().or(c.name_en.clone()).unwrap_or_default();
                                    view! {
                                        <details class="group">
                                            <summary class="list-none px-3.5 py-2.5 flex items-center gap-3 cursor-pointer hover:bg-zinc-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent-500 [&::-webkit-details-marker]:hidden">
                                                <span class="font-mono text-[0.86em] text-zinc-500 w-16 shrink-0">{c.code.clone()}</span>
                                                <span class="text-sm text-zinc-800 truncate flex-1">{name.clone()}</span>
                                                {(!unmet.is_empty()).then(|| {
                                                    let list = unmet.join(", ");
                                                    view! { <span class=format!("{TAG} {TAG_WARN} shrink-0") role="note">{move || if is_thai.get() { format!("ต้องเรียน {} ก่อน", list) } else { format!("Requires {}", list) }}</span> }
                                                })}
                                                <span class="hidden sm:inline text-xs text-zinc-500 tabular-nums shrink-0">{c.credit_pattern.clone().unwrap_or_default()}</span>
                                                <span class="text-xs text-zinc-500 shrink-0 tabular-nums">{let n = c.sections.len(); move || if is_thai.get() { format!("{n} ตอน") } else { format!("{n} sec") }}</span>
                                            </summary>
                                            <div class="px-4 pb-3 space-y-2">
                                                {c.sections.into_iter().map(|s| {
                                                    let pick = Pick { code: code.clone(), name: name.clone(), section: s.clone() };
                                                    let times = s.meetings.iter().map(|m| (m.day, format!("{}–{} {}", m.start, m.end, m.room))).collect::<Vec<_>>();
                                                    let teachers = s.meetings.first().map(|m| m.teachers.clone()).unwrap_or_default();
                                                    let picked = {
                                                        let p = pick.clone();
                                                        move || picks.get().iter().any(|x| x.code == p.code && x.section.section == p.section.section)
                                                    };
                                                    view! {
                                                        <div class="rounded-lg bg-zinc-50/60 px-3 py-2 text-xs text-zinc-600 space-y-1">
                                                            <div class="flex flex-wrap items-center gap-2">
                                                                <span class="font-semibold text-zinc-800">{format!("Section {}", s.section)}</span>
                                                                <span>{s.seats.map(|n| format!("{n} ที่นั่ง"))}</span>
                                                                <span class="text-zinc-500 truncate">{s.students.clone()}</span>
                                                                <button
                                                                    class="ml-auto text-xs font-medium px-2 py-0.5 rounded-md border border-accent-300 text-accent-700 hover:bg-accent-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                                                                    on:click={
                                                                        let pick = pick.clone();
                                                                        move |_| set_picks.update(|v| {
                                                                            if let Some(i) = v.iter().position(|x| x.code == pick.code && x.section.section == pick.section.section) {
                                                                                v.remove(i);
                                                                            } else {
                                                                                v.push(pick.clone());
                                                                            }
                                                                        })
                                                                    }
                                                                >
                                                                    {move || if picked() { t("เอาออก", "Remove") } else { t("เลือก", "Pick") }}
                                                                </button>
                                                            </div>
                                                            <div class="font-mono">
                                                                {times.is_empty().then(|| view! { <span class="text-zinc-500">{move || t("ยังไม่มีตารางเรียน", "No schedule yet")}</span> })}
                                                                {times.into_iter().map(|(day, txt)| view! {
                                                                    <span class="mr-3">{move || if is_thai.get() { DAYS_TH[day as usize % 8] } else { DAYS_EN[day as usize % 8] }}" "{txt.clone()}</span>
                                                                }).collect::<Vec<_>>()}
                                                            </div>
                                                            <div class="text-zinc-500">{teachers}</div>
                                                            <div class="text-zinc-500">
                                                                {s.midterm.clone().filter(|m| m != "-").map(|m| format!("กลางภาค {m}"))}
                                                                {s.final_exam.clone().filter(|m| m != "-").map(|m| format!(" · ปลายภาค {m}"))}
                                                            </div>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </details>
                                    }
                                }).collect::<Vec<_>>()}
                                {empty.then(|| view! { <p class="px-4 py-3 text-xs text-zinc-500">{move || t("ไม่พบรายวิชา", "No matching courses")}</p> })}
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </section>
    }
}
