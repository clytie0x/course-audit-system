//! Co-op preparation hours (อบรมเตรียมความพร้อมสหกิจศึกษา 30 ชั่วโมง): 24 hours
//! of faculty/programme activities plus 6 of university/other activities, as set
//! by the Faculty of Science report.
//!
//! That report is a Looker Studio embed whose data API needs a per-session
//! XSRF token and sends no CORS headers, so a static site cannot query it.
//! Instead the card embeds the official report for the student's cohort (the
//! student searches their ID inside it) and takes the two totals by hand;
//! values are kept in localStorage only when "save results" is on.

use leptos::prelude::*;

use crate::app::AppState;
use crate::components::ui::{CARD, SECTION_TITLE};

const REPORT_URL: &str = "https://regis.sci.psu.ac.th/activity30hr";

/// Looker Studio reports embedded on REPORT_URL, by cohort (Buddhist year).
fn report_for(cohort: u32) -> Option<&'static str> {
    Some(match cohort {
        2568 => "https://lookerstudio.google.com/embed/reporting/5fef4d4d-1390-4355-a8db-e474288de19d/page/GMqoF",
        2567 => "https://lookerstudio.google.com/embed/reporting/1163cf47-a57e-4a7b-bc0c-3bf993066912/page/oMooF",
        2566 => "https://lookerstudio.google.com/embed/reporting/3e4be6a9-e1e7-41c3-8a52-b322181fb4ff/page/e3UIE",
        2565 => "https://lookerstudio.google.com/embed/reporting/4c2412cf-3357-4e3c-999c-c48f78df4a3c/page/C7YVD",
        _ => return None,
    })
}
const KEY: &str = "coop_hours_v1";
const FACULTY_REQUIRED: f32 = 24.0;
const UNIVERSITY_REQUIRED: f32 = 6.0;

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

pub const TOTAL_REQUIRED: f32 = FACULTY_REQUIRED + UNIVERSITY_REQUIRED;

/// Hours entered by the student, shared by the co-op card and the KPI row.
#[derive(Clone, Copy)]
pub struct CoopHoursState {
    pub faculty: ReadSignal<f32>,
    pub set_faculty: WriteSignal<f32>,
    pub university: ReadSignal<f32>,
    pub set_university: WriteSignal<f32>,
}

/// Creates the hours signals (from localStorage when saved) and keeps
/// localStorage in sync with the save toggle. Call once from `App`.
pub fn provide_coop_hours(save_enabled: ReadSignal<bool>) {
    let saved = storage()
        .and_then(|s| s.get_item(KEY).ok().flatten())
        .and_then(|v| {
            let (a, b) = v.split_once(',')?;
            Some((a.parse::<f32>().ok()?, b.parse::<f32>().ok()?))
        })
        .unwrap_or((0.0, 0.0));
    let (faculty, set_faculty) = signal(saved.0);
    let (university, set_university) = signal(saved.1);

    Effect::new(move |_| {
        let value = format!("{},{}", faculty.get(), university.get());
        if let Some(s) = storage() {
            if save_enabled.get() {
                let _ = s.set_item(KEY, &value);
            } else {
                let _ = s.remove_item(KEY);
            }
        }
    });
    provide_context(CoopHoursState {
        faculty,
        set_faculty,
        university,
        set_university,
    });
}

#[component]
pub fn CoopHours() -> impl IntoView {
    let AppState {
        is_thai, cohort, ..
    } = expect_context::<AppState>();
    let CoopHoursState {
        faculty,
        set_faculty,
        university,
        set_university,
    } = expect_context::<CoopHoursState>();
    let (show_report, set_show_report) = signal(false);

    let t = move |th: &'static str, en: &'static str| if is_thai.get() { th } else { en };
    let row = move |label_th: &'static str,
                    label_en: &'static str,
                    value: ReadSignal<f32>,
                    set: WriteSignal<f32>,
                    required: f32| {
        view! {
            <label class="flex items-center gap-3">
                <span class="text-[13px] text-zinc-700 flex-1">{move || t(label_th, label_en)}</span>
                <input
                    type="number" min="0" step="1"
                    class="w-20 text-[13px] font-mono text-center px-2 py-1.5 rounded-lg bg-white border border-zinc-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                    prop:value=move || value.get().to_string()
                    on:input=move |ev| set.set(event_target_value(&ev).parse::<f32>().unwrap_or(0.0).max(0.0))
                />
                <span class=move || format!("text-xs font-semibold w-20 text-right {}", if value.get() >= required { "text-emerald-700" } else { "text-amber-700" })>
                    {move || format!("{} / {}", value.get(), required)}
                </span>
            </label>
        }
    };

    view! {
        <section class=format!("{CARD} overflow-hidden print:hidden")>
            <div class="px-4 sm:px-[18px] pt-4 pb-3 border-b border-zinc-200">
                <h2 class=SECTION_TITLE>{move || t("ชั่วโมงเตรียมความพร้อมสหกิจ 30 ชั่วโมง", "Co-op preparation hours (30)")}</h2>
                <p class="text-[13px] text-zinc-500">{move || t("เงื่อนไขก่อนลงวิชาสหกิจศึกษา", "Required before the co-op course")}</p>
            </div>
            <div class="px-4 sm:px-[18px] py-4 space-y-3">
                // Inputs stay narrow so label and value line up on a full-width card.
                <div class="max-w-xl space-y-3">
                {row("กิจกรรมของคณะ สาขา หรือหลักสูตร", "Faculty / programme activities", faculty, set_faculty, FACULTY_REQUIRED)}
                {row("กิจกรรมของมหาวิทยาลัยหรืออื่น ๆ", "University / other activities", university, set_university, UNIVERSITY_REQUIRED)}
                <p class=move || format!("text-xs font-semibold {}", if faculty.get() >= FACULTY_REQUIRED && university.get() >= UNIVERSITY_REQUIRED { "text-emerald-700" } else { "text-amber-700" })>
                    {move || {
                        let missing_f = (FACULTY_REQUIRED - faculty.get()).max(0.0);
                        let missing_u = (UNIVERSITY_REQUIRED - university.get()).max(0.0);
                        if missing_f == 0.0 && missing_u == 0.0 {
                            t("ครบเกณฑ์ 30 ชั่วโมงแล้ว", "All 30 hours done").to_string()
                        } else if is_thai.get() {
                            format!("ยังขาด: คณะ {missing_f} ชั่วโมง, มหาวิทยาลัย {missing_u} ชั่วโมง")
                        } else {
                            format!("Still needed: faculty {missing_f} h, university {missing_u} h")
                        }
                    }}
                </p>
                </div>
                // Official report for this cohort, loaded only on request (it is heavy).
                {move || cohort.get().and_then(report_for).map(|url| view! {
                    <div class="space-y-2">
                        <button
                            class="text-xs font-medium px-3 py-1.5 rounded-lg border border-accent-300 text-accent-700 hover:bg-accent-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                            on:click=move |_| set_show_report.update(|v| *v = !*v)
                        >
                            {move || {
                                let yy = cohort.get().map(|c| c % 100).unwrap_or_default();
                                match (show_report.get(), is_thai.get()) {
                                    (false, true) => format!("ดูชั่วโมงของรหัส {yy} จากรายงานของคณะ"),
                                    (false, false) => format!("Show the faculty report for cohort {yy}"),
                                    (true, true) => "ซ่อนรายงาน".to_string(),
                                    (true, false) => "Hide report".to_string(),
                                }
                            }}
                        </button>
                        {move || show_report.get().then(|| view! {
                            <p class="text-2xs text-zinc-500">{move || t("พิมพ์รหัสนักศึกษาในช่อง \"รหัสนักศึกษา\" ของรายงาน แล้วกรอกผลรวมสองช่องด้านบน", "Type your student ID into the report's search box, then enter the two totals above")}</p>
                            <iframe
                                src=url
                                title=move || t("รายงานชั่วโมงเตรียมสหกิจ", "Co-op preparation hours report")
                                class="w-full h-[560px] rounded-lg border border-zinc-200 bg-white"
                                referrerpolicy="no-referrer"
                                sandbox="allow-scripts allow-same-origin allow-popups"
                            ></iframe>
                        })}
                    </div>
                })}
                <p class="text-2xs text-zinc-500 leading-relaxed">
                    {move || t("ดูชั่วโมงของตัวเองได้ที่ ", "Look up your hours at ")}
                    <a class="text-accent-700 underline" href=REPORT_URL target="_blank" rel="noopener noreferrer">"regis.sci.psu.ac.th/activity30hr"</a>
                    {move || t(" (เลือกแท็บรหัสรุ่น แล้วค้นด้วยรหัสนักศึกษา) หรือรัน tools/sis-scraper/coop-hours.mjs", " (pick your cohort tab, search your student ID) or run tools/sis-scraper/coop-hours.mjs")}
                </p>
            </div>
        </section>
    }
}
