//! Left sidebar: PDF upload + analysis, how-it-works, save toggle, privacy note.

use crate::pdf::extract_text_from_pdf;
use audit_core::data::{curricula, curriculum_for_cohort};
use audit_core::models::ParseMode;
use audit_core::parser::{detect_cohort, parse_transcript};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::DragEvent;
use web_sys::Event;
use web_sys::HtmlInputElement;

use crate::app::AppState;

#[component]
pub fn Sidebar() -> impl IntoView {
    let AppState {
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
        curriculum_id,
        set_curriculum_id,
        parsed_courses,
        set_parsed_courses,
        set_planned_courses,
        save_enabled,
        set_save_enabled,
        is_thai,
        ..
    } = expect_context::<AppState>();
    let toggle_save_enabled = move |_| {
        let new_val = !save_enabled.get_untracked();
        set_save_enabled.set(new_val);
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("audit_save_enabled", if new_val { "1" } else { "0" });
            if !new_val {
                let _ = storage.remove_item("audit_v1");
                let _ = storage.remove_item("audit_mode_v1");
                let _ = storage.remove_item("audit_parsed_v1");
                let _ = storage.remove_item("audit_curriculum_v1");
            } else if let Some(result) = audit_result.get_untracked() {
                if let Ok(json) = serde_json::to_string(&parsed_courses.get_untracked()) {
                    let _ = storage.set_item("audit_parsed_v1", &json);
                }
                let _ = storage.set_item("audit_curriculum_v1", &curriculum_id.get_untracked());
                if let Ok(json) = serde_json::to_string(&result) {
                    let _ = storage.set_item("audit_v1", &json);
                }
                let mode = match parse_mode.get_untracked() {
                    ParseMode::Grade => "grade",
                    ParseMode::Registration => "registration",
                };
                let _ = storage.set_item("audit_mode_v1", mode);
            }
        }
    };

    // Handle file selection from input field
    let on_file_change = move |ev: Event| {
        let input = ev
            .target()
            .and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
        if let Some(input) = input {
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    set_file_name.set(file.name());
                    set_dropped_file.set(None); // file input takes precedence; clear any prior drop

                    // Revoke the previous blob URL to avoid memory leak, then create a new one
                    if let Some(old_url) = preview_url.get() {
                        let _ = web_sys::Url::revoke_object_url(&old_url);
                    }
                    // Create blob URL for PDF preview display
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&file) {
                        set_preview_url.set(Some(url));
                    }
                }
            }
        }
    };

    // Handle drag over
    let on_drag_over = move |ev: DragEvent| {
        ev.prevent_default();
    };

    // Handle drop
    let on_drop = move |ev: DragEvent| {
        ev.prevent_default();

        if let Some(data_transfer) = ev.data_transfer() {
            if let Some(files) = data_transfer.files() {
                if let Some(file) = files.get(0) {
                    if !file.name().to_lowercase().ends_with(".pdf") {
                        set_error_msg.set(Some(if is_thai.get_untracked() {
                            "กรุณาอัปโหลดไฟล์ PDF เท่านั้น".to_string()
                        } else {
                            "Please upload a PDF file.".to_string()
                        }));
                        return;
                    }
                    set_file_name.set(file.name());
                    set_dropped_file.set(Some(file.clone()));

                    // Revoke the previous blob URL to avoid memory leak, then create a new one
                    if let Some(old_url) = preview_url.get() {
                        let _ = web_sys::Url::revoke_object_url(&old_url);
                    }
                    // Create blob URL for PDF preview
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&file) {
                        set_preview_url.set(Some(url));
                    }
                }
            }
        }
    };

    // Handle start analysis
    let on_start_analysis = move |_| {
        if file_name.get().is_empty() {
            return;
        }

        set_is_loading.set(true);
        set_audit_result.set(None);
        set_error_msg.set(None);

        // Prefer file from drag-and-drop signal; fall back to the file-input element.
        // (drop events do not update the <input type="file"> files list)
        let file_opt = dropped_file.get().or_else(|| {
            web_sys::window()
                .ok_or(())
                .and_then(|w| w.document().ok_or(()))
                .and_then(|d| {
                    d.get_element_by_id("file-input")
                        .ok_or(())
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok().ok_or(()))
                })
                .ok()
                .and_then(|input| input.files())
                .and_then(|files| files.get(0))
        });

        if let Some(file) = file_opt {
            spawn_local(async move {
                use wasm_bindgen_futures::JsFuture;

                // gloo-file wraps the FileReader lifecycle cleanly: callbacks
                // are dropped when the future resolves, so no leaked closures
                // accumulate across repeated analyses.
                let gloo_file = gloo_file::File::from(file);
                let bytes = match gloo_file::futures::read_as_bytes(&gloo_file).await {
                    Ok(b) => b,
                    Err(_) => {
                        set_is_loading.set(false);
                        set_error_msg.set(Some(if is_thai.get_untracked() {
                            "ไม่สามารถอ่านไฟล์ PDF ได้ กรุณาลองใหม่อีกครั้ง".to_string()
                        } else {
                            "Failed to read the PDF file. Please try again.".to_string()
                        }));
                        return;
                    }
                };

                let uint8_array = js_sys::Uint8Array::from(bytes.as_slice());

                {
                    let promise = extract_text_from_pdf(uint8_array);
                    match JsFuture::from(promise).await {
                        Ok(text_value) => {
                            if let Some(text) = text_value.as_string() {
                                let (courses, mode) = parse_transcript(&text);
                                // Pick the curriculum revision from the cohort in the student ID.
                                if let Some(cohort) = detect_cohort(&text) {
                                    set_curriculum_id.set(curriculum_for_cohort(cohort).id.clone());
                                }
                                set_parse_mode.set(mode);
                                // Clear any prior planned courses — a fresh PDF should start clean.
                                set_planned_courses.set(Vec::new());
                                // Feed the audit effect. It'll build the AuditResult and persist
                                // to localStorage if save_enabled is on.
                                set_parsed_courses.set(courses);
                                if save_enabled.get_untracked() {
                                    if let Some(storage) = web_sys::window()
                                        .and_then(|w| w.local_storage().ok().flatten())
                                    {
                                        let mode_str = match mode {
                                            ParseMode::Grade => "grade",
                                            ParseMode::Registration => "registration",
                                        };
                                        let _ = storage.set_item("audit_mode_v1", mode_str);
                                    }
                                }
                                set_is_loading.set(false);
                            } else {
                                set_is_loading.set(false);
                                set_error_msg.set(Some(if is_thai.get_untracked() {
                                                "ไม่สามารถดึงข้อความจาก PDF กรุณาตรวจสอบว่าเป็นใบแสดงผลการเรียนที่ถูกต้อง".to_string()
                                            } else {
                                                "Could not extract text from the PDF. Make sure it's a valid transcript.".to_string()
                                            }));
                            }
                        }
                        Err(_e) => {
                            set_is_loading.set(false);
                            set_error_msg.set(Some(if is_thai.get_untracked() {
                                "การดึงข้อมูล PDF ล้มเหลว ไฟล์อาจเสียหายหรือถูกเข้ารหัส".to_string()
                            } else {
                                "PDF extraction failed. The file may be corrupted or encrypted."
                                    .to_string()
                            }));
                        }
                    }
                }
            });
        }
    };

    view! {

        // Upload Card
        <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft p-5 flex flex-col gap-4">
            <div>
                <h2 class="text-base font-semibold text-zinc-900 dark:text-zinc-100 tracking-tight">{move || if is_thai.get() { "อัปโหลดใบแสดงผลการเรียน" } else { "Upload Transcript" }}</h2>
                <p class="text-[13px] text-zinc-500 dark:text-zinc-400 mt-0.5 leading-relaxed">{move || if is_thai.get() { "PDF ของคุณถูกประมวลผลในเบราว์เซอร์ทั้งหมด ข้อมูลไม่ออกจากอุปกรณ์ของคุณ" } else { "Your PDF is processed entirely in the browser. Nothing leaves your device." }}</p>
            </div>

            // Drop zone
            <div
                class="group relative border border-dashed border-zinc-300 dark:border-zinc-700 rounded-xl p-5 text-center bg-zinc-50/50 dark:bg-zinc-800/30 hover:border-brand-400 hover:bg-brand-50/30 dark:hover:border-brand-500 dark:hover:bg-brand-950/20 transition-all duration-200 cursor-pointer focus-within:border-brand-500 focus-within:ring-2 focus-within:ring-brand-500/30"
                on:dragover=on_drag_over
                on:drop=on_drop
            >
                <input
                    type="file"
                    accept="application/pdf"
                    class="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
                    id="file-input"
                    aria-label={move || if is_thai.get() { "อัปโหลดใบแสดงผลการเรียน PDF" } else { "Upload transcript PDF" }}
                    on:change=on_file_change
                />
                <div class="flex flex-col items-center gap-2.5 pointer-events-none">
                    <div class="w-10 h-10 rounded-xl bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700 text-zinc-400 dark:text-zinc-500 flex items-center justify-center group-hover:text-brand-500 group-hover:border-brand-200 group-hover:bg-brand-50 dark:group-hover:bg-brand-950/40 transition-colors">
                        <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5"/>
                        </svg>
                    </div>
                    <div>
                        <p class="text-sm font-medium text-zinc-700 dark:text-zinc-300 group-hover:text-brand-600 dark:group-hover:text-brand-400 transition-colors">{move || if is_thai.get() { "วางไฟล์ PDF ที่นี่ หรือคลิกเพื่อเลือกไฟล์" } else { "Drop PDF here or click to browse" }}</p>
                        <p class="text-2xs text-zinc-400 dark:text-zinc-500 mt-0.5">{move || if is_thai.get() { "รับเฉพาะไฟล์ .pdf เท่านั้น" } else { "Accepts .pdf files only" }}</p>
                    </div>
                </div>
            </div>

            // Selected file
            {move || (!file_name.get().is_empty()).then(|| view! {
                <div class="flex items-center gap-2.5 px-3 py-2.5 rounded-lg bg-emerald-50 border border-emerald-200/60 animate-scale-in">
                    <svg class="w-4 h-4 text-emerald-500 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    <p class="text-[13px] text-emerald-800 font-medium truncate">{file_name.get()}</p>
                </div>
            })}

            // Analyze button
            <button
                class="w-full flex items-center justify-center gap-2 bg-zinc-900 hover:bg-zinc-800 dark:bg-brand-600 dark:hover:bg-brand-500 text-white text-sm font-medium py-2.5 px-4 rounded-xl transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed active:scale-[0.98] shadow-soft hover:shadow-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
                disabled={move || file_name.get().is_empty() || is_loading.get()}
                on:click=on_start_analysis
            >
                {move || if is_loading.get() {
                    view! {
                        <svg class="animate-spin h-4 w-4 text-white/70" aria-hidden="true" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/></svg>
                        <span>{move || if is_thai.get() { "กำลังวิเคราะห์..." } else { "Analyzing..." }}</span>
                    }.into_any()
                } else {
                    view! {
                        <span>{move || if is_thai.get() { "วิเคราะห์ใบแสดงผลการเรียน" } else { "Analyze Transcript" }}</span>
                        <svg class="w-4 h-4 opacity-50" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3"/></svg>
                    }.into_any()
                }}
            </button>

            // Curriculum revision (auto-picked from the student ID after analysis)
            <label class="flex flex-col gap-1.5">
                <span class="text-2xs font-semibold text-zinc-500 dark:text-zinc-400 uppercase tracking-widest">{move || if is_thai.get() { "หลักสูตร" } else { "Curriculum" }}</span>
                <select
                    class="text-[13px] px-3 py-2 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
                    prop:value=move || curriculum_id.get()
                    on:change=move |ev| set_curriculum_id.set(event_target_value(&ev))
                >
                    {curricula().iter().map(|c| view! { <option value=c.id.clone()>{c.name.clone()}</option> }).collect::<Vec<_>>()}
                </select>
            </label>
        </div>

        // How it works card (only when no file selected)
        {move || preview_url.get().is_none().then(|| view! {
            <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft p-5 animate-fade-in-up">
                <h3 class="text-xs font-semibold text-zinc-500 dark:text-zinc-400 uppercase tracking-widest mb-3">{move || if is_thai.get() { "วิธีการใช้งาน" } else { "How it works" }}</h3>
                <div class="space-y-3">
                    <div class="flex items-start gap-3">
                        <div class="mt-0.5 w-5 h-5 shrink-0 rounded-full bg-zinc-900 dark:bg-brand-600 text-white flex items-center justify-center text-2xs font-bold">"1"</div>
                        <p class="text-[13px] text-zinc-600 dark:text-zinc-400 leading-relaxed">{move || if is_thai.get() { "ดาวน์โหลดใบแสดงผลการเรียน PDF จากระบบ SIS" } else { "Download your unofficial transcript PDF from SIS." }}</p>
                    </div>
                    <div class="flex items-start gap-3">
                        <div class="mt-0.5 w-5 h-5 shrink-0 rounded-full bg-zinc-900 dark:bg-brand-600 text-white flex items-center justify-center text-2xs font-bold">"2"</div>
                        <p class="text-[13px] text-zinc-600 dark:text-zinc-400 leading-relaxed">{move || if is_thai.get() { "อัปโหลดไฟล์ PDF ผ่านพื้นที่วางไฟล์ด้านบน" } else { "Upload the PDF using the drop zone above." }}</p>
                    </div>
                    <div class="flex items-start gap-3">
                        <div class="mt-0.5 w-5 h-5 shrink-0 rounded-full bg-zinc-900 dark:bg-brand-600 text-white flex items-center justify-center text-2xs font-bold">"3"</div>
                        <p class="text-[13px] text-zinc-600 dark:text-zinc-400 leading-relaxed">{move || if is_thai.get() { "คลิก วิเคราะห์ — ทุกอย่างประมวลผลบนเครื่องของคุณเพื่อความเป็นส่วนตัวสูงสุด" } else { "Click Analyze — everything is processed locally for maximum privacy." }}</p>
                    </div>
                </div>
            </div>
        })}

        // Save session toggle (opt-in localStorage)
        <label class="flex items-center gap-3 px-4 py-3 rounded-xl bg-white dark:bg-zinc-900 border border-zinc-200/80 dark:border-zinc-800 cursor-pointer hover:border-brand-300 dark:hover:border-brand-700 transition-colors">
            <input
                type="checkbox"
                class="w-4 h-4 rounded border-zinc-300 dark:border-zinc-600 text-brand-600 focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
                prop:checked=move || save_enabled.get()
                on:change=toggle_save_enabled
            />
            <div class="flex-1 min-w-0">
                <p class="text-[13px] font-medium text-zinc-800 dark:text-zinc-200">{move || if is_thai.get() { "บันทึกผลในเบราว์เซอร์" } else { "Save results in browser" }}</p>
                <p class="text-2xs text-zinc-500 dark:text-zinc-400 mt-0.5 leading-relaxed">{move || if is_thai.get() { "ผลลัพธ์จะอยู่ใน localStorage (ไม่ส่งออกนอกเครื่อง)" } else { "Kept in localStorage only — never leaves your device." }}</p>
            </div>
        </label>

        // Privacy badge
        <div class="hidden lg:flex items-center gap-2 px-4 py-3 rounded-xl bg-zinc-50 dark:bg-zinc-900/60 border border-zinc-100 dark:border-zinc-800">
            <svg class="w-4 h-4 text-zinc-400 dark:text-zinc-500 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z"/>
            </svg>
            <p class="text-2xs text-zinc-500 dark:text-zinc-400 font-medium">{move || if is_thai.get() { "ใบแสดงผลการเรียนของคุณไม่ออกจากอุปกรณ์นี้ ทุกอย่างประมวลผลใน WebAssembly" } else { "Your transcript never leaves this device. All processing runs in WebAssembly." }}</p>
        </div>
    }
}
