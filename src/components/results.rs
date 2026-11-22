//! Results view: export/print toolbar, registration banner and the result sections.

use crate::components::category_card::CategoryCard;
use crate::components::category_summary::CategorySummary;
use crate::components::coop_hours::CoopHours;
use crate::components::hero::HeroCard;
use crate::components::missing_requirements::MissingRequirements;
use crate::components::offerings::OfferingsPanel;
use crate::components::semester_breakdown::SemesterBreakdown;
use crate::components::what_if::WhatIfSimulator;
use audit_core::models::AuditResult;
use audit_core::models::ParseMode;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::app::AppState;

#[component]
pub fn ResultsView(result: AuditResult) -> impl IntoView {
    let AppState {
        audit_result,
        parse_mode,
        is_thai,
        ..
    } = expect_context::<AppState>();
    // Download the AuditResult as a JSON file.
    let export_json = move |_| {
        let Some(result) = audit_result.get_untracked() else {
            return;
        };
        let Ok(json) = serde_json::to_string_pretty(&result) else {
            return;
        };
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        // Build a Blob containing the JSON text and stream it via <a download>.
        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(&json));
        let Ok(blob) = web_sys::Blob::new_with_str_sequence(&parts) else {
            return;
        };
        let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
            return;
        };
        if let Ok(anchor) = document.create_element("a") {
            let _ = anchor.set_attribute("href", &url);
            let _ = anchor.set_attribute("download", "course-audit.json");
            if let Some(html_anchor) = anchor.dyn_ref::<web_sys::HtmlElement>() {
                html_anchor.click();
            }
        }
        let _ = web_sys::Url::revoke_object_url(&url);
    };

    // Trigger the browser's print dialog; style.css provides a print stylesheet.
    let print_report = move |_| {
        if let Some(window) = web_sys::window() {
            let _ = window.print();
        }
    };

    view! {
        <div class="space-y-5 animate-fade-in">

            // ── Actions toolbar (export / print) ─────
            <div class="flex items-center justify-end gap-2 print:hidden">
                <button
                    class="inline-flex items-center gap-1.5 text-xs font-medium px-3 py-1.5 rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 text-zinc-700 dark:text-zinc-300 hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-950"
                    on:click=export_json
                >
                    <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3"/></svg>
                    <span>{move || if is_thai.get() { "ดาวน์โหลด JSON" } else { "Export JSON" }}</span>
                </button>
                <button
                    class="inline-flex items-center gap-1.5 text-xs font-medium px-3 py-1.5 rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 text-zinc-700 dark:text-zinc-300 hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-950"
                    on:click=print_report
                >
                    <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5zm-3 0h.008v.008H15V10.5z"/></svg>
                    <span>{move || if is_thai.get() { "พิมพ์ / บันทึก PDF" } else { "Print / Save PDF" }}</span>
                </button>
            </div>

            // ── Registration-mode banner ─────────────
            {move || (parse_mode.get() == ParseMode::Registration).then(|| view! {
                <div class="rounded-2xl border border-amber-200/60 dark:border-amber-900/40 bg-amber-50 dark:bg-amber-950/30 px-5 py-3 flex items-start gap-3" role="status">
                    <svg class="w-5 h-5 text-amber-500 shrink-0 mt-0.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m0-10.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285zM12 15.75h.007v.008H12v-.008z"/></svg>
                    <div class="text-[13px] text-amber-900 dark:text-amber-200 leading-relaxed">
                        <p class="font-semibold">{move || if is_thai.get() { "กำลังดูไฟล์ผลการลงทะเบียน" } else { "Viewing registration file" }}</p>
                        <p class="text-amber-800/90 dark:text-amber-300/90 mt-0.5">{move || if is_thai.get() { "วิชาที่กำลังเรียนแสดงเป็นแท็กสีเหลือง และยังไม่นับหน่วยกิตจนกว่าจะได้เกรด" } else { "In-progress courses appear with a yellow tag and don't count toward credits until graded." }}</p>
                    </div>
                </div>
            })}

            <HeroCard result=result.clone()/>
            <CategorySummary result=result.clone()/>

            // ── Course Details Accordion ─────────────
            <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft overflow-hidden">
                <div class="px-5 py-4 border-b border-zinc-100 dark:border-zinc-800 flex items-center gap-2.5">
                    <svg class="w-4 h-4 text-zinc-400 dark:text-zinc-500" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 12h16.5m-16.5 3.75h16.5M3.75 19.5h16.5M5.625 4.5h12.75a1.875 1.875 0 010 3.75H5.625a1.875 1.875 0 010-3.75z"/></svg>
                    <h3 class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{move || if is_thai.get() { "รายละเอียดวิชา" } else { "Course Details" }}</h3>
                </div>
                <div class="divide-y divide-zinc-100 dark:divide-zinc-800">
                    {result.categories.iter().map(|category| {
                        let category = category.clone();
                        view! { <CategoryCard category={category} /> }
                    }).collect::<Vec<_>>()}
                </div>
            </div>

            <WhatIfSimulator/>
            <OfferingsPanel/>
            <CoopHours/>
            <SemesterBreakdown result=result.clone()/>
            <MissingRequirements result=result.clone()/>
        </div>
    }
}
