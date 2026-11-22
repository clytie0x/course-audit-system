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
        <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft overflow-hidden print:hidden">
            <div class="px-5 py-4 border-b border-zinc-100 dark:border-zinc-800 flex flex-wrap items-center gap-2.5">
                <svg class="w-4 h-4 text-brand-500" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5"/></svg>
                <h3 class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{move || t("รายวิชาที่เปิดสอน", "Course offerings")}</h3>
                <span class="text-2xs text-zinc-500 dark:text-zinc-400">
                    {move || offerings.get().flatten().map(|d| format!("SIS · {} · {}", d.campus, d.scraped_at))}
                </span>
            </div>
            <div class="p-5 space-y-4">
                <div class="grid grid-cols-1 sm:grid-cols-[auto_1fr_auto] gap-2.5 items-center">
                    <select
                        class="text-[13px] font-mono px-2 py-2 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
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
                        class="text-[13px] px-3 py-2 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
                        prop:value=move || query.get()
                        on:input=move |ev| set_query.set(event_target_value(&ev))
                    />
                    <label class="inline-flex items-center gap-2 text-xs text-zinc-600 dark:text-zinc-300 cursor-pointer">
                        <input type="checkbox" class="w-4 h-4 rounded"
                            prop:checked=move || only_outstanding.get()
                            on:change=move |ev| set_only_outstanding.set(event_target_checked(&ev))
                        />
                        {move || t("เฉพาะวิชาบังคับที่ยังไม่ได้เรียน", "Required courses not yet taken")}
                    </label>
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
                            <div class="rounded-xl border border-brand-200/70 dark:border-brand-900/50 bg-brand-50/40 dark:bg-brand-950/20 p-4 space-y-2.5">
                                <div class="flex items-center justify-between">
                                    <p class="text-xs font-semibold text-brand-700 dark:text-brand-300">{move || t("ตารางเรียนของฉัน", "My timetable")}</p>
                                    <button class="text-2xs font-semibold text-zinc-500 hover:text-red-600" on:click=move |_| set_picks.set(Vec::new())>
                                        {move || t("ล้าง", "Clear")}
                                    </button>
                                </div>
                                <ul class="space-y-1">
                                    {slots.into_iter().map(|(day, time, what)| view! {
                                        <li class="text-[13px] font-mono text-zinc-700 dark:text-zinc-300">
                                            <span class="inline-block w-10 text-zinc-500">{move || if is_thai.get() { DAYS_TH[day as usize % 8] } else { DAYS_EN[day as usize % 8] }}</span>
                                            <span class="inline-block w-28">{time}</span>
                                            {what}
                                        </li>
                                    }).collect::<Vec<_>>()}
                                </ul>
                                {(!clashes.is_empty()).then(|| view! {
                                    <ul class="space-y-1" role="alert">
                                        {clashes.into_iter().map(|c| view! {
                                            <li class="text-xs font-medium text-red-700 dark:text-red-300">{format!("⚠ {c}")}</li>
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
                            <div class="divide-y divide-zinc-100 dark:divide-zinc-800 rounded-lg border border-zinc-200/80 dark:border-zinc-800">
                                {rows.into_iter().map(|c| {
                                    let unmet = unmet_prerequisites(&c.code, &done, cur);
                                    let code = c.code.clone();
                                    let name = c.name_th.clone().or(c.name_en.clone()).unwrap_or_default();
                                    view! {
                                        <details class="group">
                                            <summary class="list-none px-4 py-2.5 flex items-center gap-3 cursor-pointer hover:bg-zinc-50/80 dark:hover:bg-zinc-800/50">
                                                <span class="font-mono text-2xs font-semibold text-zinc-500 w-16 shrink-0">{c.code.clone()}</span>
                                                <span class="text-[13px] text-zinc-800 dark:text-zinc-200 truncate flex-1">{name.clone()}</span>
                                                {(!unmet.is_empty()).then(|| {
                                                    let list = unmet.join(", ");
                                                    view! { <span class="text-2xs font-semibold px-1.5 py-0.5 rounded bg-amber-50 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300" role="note">{move || if is_thai.get() { format!("ต้องเรียน {} ก่อน", list) } else { format!("Requires {}", list) }}</span> }
                                                })}
                                                <span class="text-2xs text-zinc-400 font-mono shrink-0">{c.credit_pattern.clone().unwrap_or_default()}</span>
                                                <span class="text-2xs text-zinc-400 shrink-0">{format!("{} sec", c.sections.len())}</span>
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
                                                        <div class="rounded-lg bg-zinc-50/60 dark:bg-zinc-800/40 px-3 py-2 text-xs text-zinc-600 dark:text-zinc-300 space-y-1">
                                                            <div class="flex flex-wrap items-center gap-2">
                                                                <span class="font-semibold text-zinc-800 dark:text-zinc-100">{format!("Section {}", s.section)}</span>
                                                                <span>{s.seats.map(|n| format!("{n} ที่นั่ง"))}</span>
                                                                <span class="text-zinc-500 truncate">{s.students.clone()}</span>
                                                                <button
                                                                    class="ml-auto text-2xs font-semibold px-2 py-0.5 rounded border border-brand-300 text-brand-700 dark:text-brand-300 hover:bg-brand-50 dark:hover:bg-brand-950/40"
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
                                                                {times.is_empty().then(|| view! { <span class="text-zinc-400">{move || t("ยังไม่มีตารางเรียน", "No schedule yet")}</span> })}
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
        </div>
    }
}
