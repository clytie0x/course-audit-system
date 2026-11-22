//! Top-level `App` Leptos component.
//!
//! Owns the shared UI state (upload, audit result, parse mode, persistence,
//! What-If plan), provides it to `components::*` as `AppState`, and
//! orchestrates reactive re-audits via `audit_core::auditor::build_audit`.
//! Section markup and handlers live in `src/components/`.

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Title};

use crate::components::header::Header;
use crate::components::offerings::OfferingsPanel;
use crate::components::results::ResultsView;
use crate::components::sidebar::Sidebar;
use audit_core::auditor::build_audit;
use audit_core::data::{curricula, curriculum};
use audit_core::models::{AuditResult, ParseMode, ParsedCourse};

/// Signals shared by the UI components, provided once by `App` via context.
#[derive(Clone, Copy)]
pub struct AppState {
    pub file_name: ReadSignal<String>,
    pub set_file_name: WriteSignal<String>,
    pub preview_url: ReadSignal<Option<String>>,
    pub set_preview_url: WriteSignal<Option<String>>,
    pub audit_result: ReadSignal<Option<AuditResult>>,
    pub set_audit_result: WriteSignal<Option<AuditResult>>,
    pub is_loading: ReadSignal<bool>,
    pub set_is_loading: WriteSignal<bool>,
    pub set_error_msg: WriteSignal<Option<String>>,
    pub dropped_file: ReadSignal<Option<web_sys::File>, LocalStorage>,
    pub set_dropped_file: WriteSignal<Option<web_sys::File>, LocalStorage>,
    pub parse_mode: ReadSignal<ParseMode>,
    pub set_parse_mode: WriteSignal<ParseMode>,
    pub parsed_courses: ReadSignal<Vec<ParsedCourse>>,
    pub set_parsed_courses: WriteSignal<Vec<ParsedCourse>>,
    pub planned_courses: ReadSignal<Vec<ParsedCourse>>,
    pub set_planned_courses: WriteSignal<Vec<ParsedCourse>>,
    pub plan_code: ReadSignal<String>,
    pub set_plan_code: WriteSignal<String>,
    pub plan_name: ReadSignal<String>,
    pub set_plan_name: WriteSignal<String>,
    pub plan_credit: ReadSignal<f32>,
    pub set_plan_credit: WriteSignal<f32>,
    pub plan_grade: ReadSignal<String>,
    pub set_plan_grade: WriteSignal<String>,
    pub curriculum_id: ReadSignal<String>,
    /// Admission cohort from the student ID on the analysed PDF (e.g. 2567).
    pub cohort: ReadSignal<Option<u32>>,
    pub set_cohort: WriteSignal<Option<u32>>,
    pub set_curriculum_id: WriteSignal<String>,
    pub save_enabled: ReadSignal<bool>,
    pub set_save_enabled: WriteSignal<bool>,
    pub is_thai: ReadSignal<bool>,
    pub set_is_thai: WriteSignal<bool>,
}

/// Main application component
///
/// Manages state for file upload, PDF preview, audit results, and loading state.
/// Implements drag-and-drop PDF upload with real-time PDF preview.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // State management for file upload and audit processing
    let (file_name, set_file_name) = signal(String::new());
    let (preview_url, set_preview_url) = signal(Option::<String>::None);
    let (audit_result, set_audit_result) = signal(Option::<AuditResult>::None);
    let (is_loading, set_is_loading) = signal(false);
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    // Stores the file from drag-and-drop (file input is not updated by drop events)
    let (dropped_file, set_dropped_file) = signal_local(Option::<web_sys::File>::None);
    // Which PDF format was detected during parsing (drives the mode banner).
    let (parse_mode, set_parse_mode) = signal(ParseMode::Grade);
    // Raw parsed courses from the PDF (kept around so the What-If simulator
    // can re-run audits without re-parsing).
    let (parsed_courses, set_parsed_courses) = signal(Vec::<ParsedCourse>::new());
    // User-added hypothetical courses appended to the audit input.
    let (planned_courses, set_planned_courses) = signal(Vec::<ParsedCourse>::new());
    // Form fields for the simulator
    let (plan_code, set_plan_code) = signal(String::new());
    let (plan_name, set_plan_name) = signal(String::new());
    let (plan_credit, set_plan_credit) = signal(3.0_f32);
    let (plan_grade, set_plan_grade) = signal("A".to_string());

    // Persistence — opt-in localStorage of the last audit result (key "audit_v1").
    // Remains off by default to stay privacy-first.
    let (save_enabled, set_save_enabled) = signal(false);
    // Curriculum revision the audit runs against (see audit_core::data).
    let (curriculum_id, set_curriculum_id) = signal(curricula()[0].id.clone());
    let (cohort, set_cohort) = signal(None::<u32>);

    // Try to rehydrate both the toggle and a prior result on mount.
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(Some(flag)) = storage.get_item("audit_save_enabled") {
            if flag == "1" {
                set_save_enabled.set(true);
                if let Ok(Some(raw)) = storage.get_item("audit_v1") {
                    if let Ok(saved) = serde_json::from_str::<AuditResult>(&raw) {
                        set_audit_result.set(Some(saved));
                    }
                }
                // The transcript itself, so What-If and curriculum changes re-audit it
                // (older saves only have the result above).
                if let Ok(Some(raw)) = storage.get_item("audit_parsed_v1") {
                    if let Ok(saved) = serde_json::from_str::<Vec<ParsedCourse>>(&raw) {
                        set_parsed_courses.set(saved);
                    }
                }
                if let Ok(Some(id)) = storage.get_item("audit_curriculum_v1") {
                    set_curriculum_id.set(id);
                }
                if let Ok(Some(c)) = storage.get_item("audit_cohort_v1") {
                    set_cohort.set(c.parse().ok());
                }
                if let Ok(Some(mode_raw)) = storage.get_item("audit_mode_v1") {
                    if mode_raw == "registration" {
                        set_parse_mode.set(ParseMode::Registration);
                    }
                }
            }
        }
    }

    // Re-run the audit whenever the parsed transcript or the simulator's
    // planned courses change. Skip when nothing has been parsed yet so the
    // empty-state keeps rendering.
    Effect::new(move |_| {
        let parsed = parsed_courses.get();
        let planned = planned_courses.get();
        if parsed.is_empty() && planned.is_empty() {
            return;
        }
        let cur_id = curriculum_id.get();
        if save_enabled.get_untracked() {
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            {
                if let Ok(json) = serde_json::to_string(&parsed) {
                    let _ = storage.set_item("audit_parsed_v1", &json);
                }
                let _ = storage.set_item("audit_curriculum_v1", &cur_id);
                if let Some(c) = cohort.get_untracked() {
                    let _ = storage.set_item("audit_cohort_v1", &c.to_string());
                }
            }
        }
        let mut combined: Vec<ParsedCourse> = parsed;
        combined.extend(planned);
        let result = build_audit(&combined, curriculum(&cur_id));
        if save_enabled.get_untracked() {
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            {
                if let Ok(json) = serde_json::to_string(&result) {
                    let _ = storage.set_item("audit_v1", &json);
                }
            }
        }
        set_audit_result.set(Some(result));
    });

    // Language toggle: true = Thai (default), false = English
    let (is_thai, set_is_thai) = signal(true);
    provide_context(is_thai);
    provide_context(AppState {
        file_name,
        set_file_name,
        preview_url,
        set_preview_url,
        audit_result,
        set_audit_result,
        is_loading,
        set_is_loading,
        set_error_msg,
        dropped_file,
        set_dropped_file,
        parse_mode,
        set_parse_mode,
        parsed_courses,
        set_parsed_courses,
        planned_courses,
        set_planned_courses,
        plan_code,
        set_plan_code,
        plan_name,
        set_plan_name,
        plan_credit,
        set_plan_credit,
        plan_grade,
        set_plan_grade,
        curriculum_id,
        set_curriculum_id,
        cohort,
        set_cohort,
        save_enabled,
        set_save_enabled,
        is_thai,
        set_is_thai,
    });

    view! {
        <Title text="Course Audit — PSU CS"/>

        <div class="min-h-screen font-sans text-zinc-900 dark:text-zinc-100 flex flex-col selection:bg-brand-100 dark:selection:bg-brand-900/60">

            <Header/>

            // ── Main Content ────────────────────────────────────────────
            <div class="flex-1 flex flex-col lg:flex-row max-w-[1440px] w-full mx-auto p-4 sm:p-6 gap-5 animate-fade-in">

                // ── Left Sidebar ────────────────────────────────────────
                <aside class="w-full lg:w-[360px] shrink-0 flex flex-col gap-4">
                    <Sidebar/>
                </aside>

                // ── Right Panel — Results ────────────────────────────────
                <main class="flex-1 min-w-0" aria-live="polite" aria-atomic="false">
                    {move || {
                        if is_loading.get() {
                            // Loading state
                            view! {
                                <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft h-full min-h-[500px] flex flex-col items-center justify-center gap-4" role="status" aria-live="polite">
                                    <div class="relative">
                                        <div class="w-12 h-12 rounded-full border-2 border-zinc-200 dark:border-zinc-700"></div>
                                        <div class="absolute inset-0 w-12 h-12 rounded-full border-2 border-brand-500 border-t-transparent animate-spin"></div>
                                    </div>
                                    <div class="text-center">
                                        <p class="text-sm font-medium text-zinc-700 dark:text-zinc-300">{move || if is_thai.get() { "กำลังวิเคราะห์ใบแสดงผลการเรียน..." } else { "Analyzing transcript..." }}</p>
                                        <p class="text-xs text-zinc-400 dark:text-zinc-500 mt-1">{move || if is_thai.get() { "กำลังดึงข้อมูลวิชาและตรวจสอบข้อกำหนด" } else { "Parsing courses and validating requirements" }}</p>
                                    </div>
                                </div>
                            }.into_any()
                        } else if let Some(err) = error_msg.get() {
                            // Error state
                            view! {
                                <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft h-full min-h-[500px] flex flex-col items-center justify-center gap-4 px-8 text-center" role="alert">
                                    <div class="w-12 h-12 rounded-full bg-red-50 dark:bg-red-950/40 flex items-center justify-center">
                                        <svg class="w-6 h-6 text-red-500" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z"/>
                                        </svg>
                                    </div>
                                    <div>
                                        <p class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{move || if is_thai.get() { "การวิเคราะห์ล้มเหลว" } else { "Analysis Failed" }}</p>
                                        <p class="text-[13px] text-zinc-500 dark:text-zinc-400 mt-1 max-w-sm leading-relaxed">{err}</p>
                                    </div>
                                </div>
                            }.into_any()
                        } else if let Some(result) = audit_result.get() {
                            // Results view
                            view! { <ResultsView result=result/> }.into_any()
                        } else {
                            // Empty state (offerings stay searchable without a transcript)
                            view! {
                                <div class="space-y-5">
                                <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft min-h-[320px] flex flex-col items-center justify-center gap-5 px-8 text-center">
                                    <div class="w-16 h-16 rounded-2xl bg-zinc-50 dark:bg-zinc-800/50 border border-zinc-200 dark:border-zinc-700 flex items-center justify-center">
                                        <svg class="w-7 h-7 text-zinc-300 dark:text-zinc-600" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z"/>
                                        </svg>
                                    </div>
                                    <div>
                                        <h3 class="text-base font-semibold text-zinc-800 dark:text-zinc-200 mb-1">{move || if is_thai.get() { "ยังไม่ได้อัปโหลดใบแสดงผลการเรียน" } else { "No transcript uploaded" }}</h3>
                                        <p class="text-sm text-zinc-400 dark:text-zinc-500 max-w-xs leading-relaxed">{move || if is_thai.get() { "อัปโหลดใบแสดงผลการเรียน PDF เพื่อดูความคืบหน้าในการสำเร็จการศึกษา" } else { "Upload your PDF transcript to see a breakdown of your progress toward graduation." }}</p>
                                    </div>
                                </div>
                                <OfferingsPanel/>
                                </div>
                            }.into_any()
                        }
                    }}
                </main>
            </div>
        </div>
    }
}
