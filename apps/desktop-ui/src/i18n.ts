import { get, writable } from 'svelte/store';

export type AppLocale = 'en' | 'ar';

const STORAGE_KEY = 'scriptotar.uiLanguage';
const TRANSLATED_ATTRIBUTES = ['aria-label', 'placeholder', 'title'] as const;

function readStoredLocale(): AppLocale {
  if (typeof window === 'undefined') return 'en';
  try {
    return window.localStorage.getItem(STORAGE_KEY) === 'ar' ? 'ar' : 'en';
  } catch {
    return 'en';
  }
}

export const locale = writable<AppLocale>(readStoredLocale());

const arabic: Record<string, string> = {
  'Primary navigation': 'التنقل الرئيسي',
  'Creator workstation': 'محطة عمل صانع المحتوى',
  Dashboard: 'لوحة التحكم',
  Research: 'البحث',
  Jobs: 'المهام',
  Transcript: 'النص المفرغ',
  'Transcript workspace': 'مساحة النصوص المفرغة',
  'AI Studio': 'استوديو الذكاء الاصطناعي',
  Library: 'المكتبة',
  Settings: 'الإعدادات',
  'Local-first': 'محلي أولاً',
  'Rust-owned workspace state': 'حالة مساحة العمل يديرها Rust',
  Project: 'المشروع',
  Language: 'اللغة',
  'Interface language': 'لغة الواجهة',
  English: 'الإنجليزية',
  Arabic: 'العربية',
  'Global search': 'البحث العام',
  'Search workspace': 'البحث في مساحة العمل',
  'Search transcripts, projects, creators…': 'ابحث في النصوص والمشاريع وصناع المحتوى…',
  'Workspace search results': 'نتائج البحث في مساحة العمل',
  'No local matches.': 'لا توجد نتائج محلية.',
  'Open jobs': 'فتح المهام',
  Idle: 'خامل',
  'Opening Scriptotar': 'جارٍ فتح Scriptotar',
  'Loading your local workspace…': 'جارٍ تحميل مساحة العمل المحلية…',
  'Could not open Scriptotar': 'تعذر فتح Scriptotar',
  'Could not switch projects': 'تعذر تبديل المشروع',
  'Background status unavailable': 'حالة العمليات الخلفية غير متاحة',
  'Unable to load the desktop workspace.': 'تعذر تحميل مساحة عمل سطح المكتب.',
  'Could not refresh background operation status.': 'تعذر تحديث حالة العمليات الخلفية.',
  'Unable to switch projects.': 'تعذر تبديل المشاريع.',
  'Something went wrong': 'حدث خطأ ما',
  'Try again': 'حاول مجدداً',

  'Workspace overview': 'نظرة عامة على مساحة العمل',
  'Creator research, transcripts, and AI work in one local workspace.': 'بحث صناع المحتوى والنصوص المفرغة وأعمال الذكاء الاصطناعي في مساحة محلية واحدة.',
  'New research scan': 'بدء بحث جديد',
  'Project summary': 'ملخص المشروع',
  'Library items': 'عناصر المكتبة',
  'across research + transcripts': 'من البحث والنصوص المفرغة',
  'Active jobs': 'المهام النشطة',
  'queue keeps moving after failures': 'يستمر الطابور حتى بعد حالات الفشل',
  'Recent creators': 'صناع المحتوى مؤخراً',
  'AI work': 'أعمال الذكاء الاصطناعي',
  'prompt and BYOK runs': 'تشغيل المطالبات وBYOK',
  Activity: 'النشاط',
  'Current jobs': 'المهام الحالية',
  'Open queue →': 'فتح الطابور ←',
  Signals: 'الإشارات',
  Watching: 'قيد المتابعة',
  'Transcript library': 'مكتبة النصوص المفرغة',
  'Recent transcripts': 'أحدث النصوص المفرغة',
  'Open →': 'فتح ←',
  'Recent work': 'أحدث الأعمال',
  'Create →': 'إنشاء ←',
  'Copy Prompt': 'نسخ المطالبة',

  All: 'الكل',
  Active: 'نشطة',
  'Needs attention': 'تحتاج انتباهاً',
  Finished: 'منتهية',
  'Video selected. Queue it when ready.': 'تم اختيار الفيديو. أضفه إلى الطابور عندما تكون جاهزاً.',
  'Persistent activity': 'نشاط مستمر',
  'Choose local media with the desktop picker or queue a supported URL. Rust still validates every path and owns queue state.': 'اختر وسائط محلية عبر نافذة سطح المكتب أو أضف رابطاً مدعوماً إلى الطابور. يتحقق Rust من كل مسار ويدير حالة الطابور.',
  'Add transcription job': 'إضافة مهمة تفريغ',
  'Local media': 'وسائط محلية',
  'No video selected': 'لم يتم اختيار فيديو',
  'Use the native desktop picker for normal operation.': 'استخدم نافذة اختيار الملفات الأصلية في سطح المكتب للاستخدام العادي.',
  'Choose video': 'اختيار فيديو',
  'Queue selected': 'إضافة المحدد للطابور',
  'Supported media URL': 'رابط وسائط مدعوم',
  'Media URL': 'رابط الوسائط',
  'Queue URL': 'إضافة الرابط للطابور',
  'Advanced: enter a local path manually': 'متقدم: أدخل مساراً محلياً يدوياً',
  'Manual local media path': 'مسار الوسائط المحلي يدوياً',
  'Queue path': 'إضافة المسار للطابور',
  'URL queued.': 'تمت إضافة الرابط إلى الطابور.',
  'Job filters': 'مرشحات المهام',
  'No jobs in this view': 'لا توجد مهام في هذا العرض',
  'Change the filter or queue media above.': 'غيّر المرشح أو أضف وسائط إلى الطابور أعلاه.',
  Cancel: 'إلغاء',
  Retry: 'إعادة المحاولة',
  'Open transcript': 'فتح النص المفرغ',
  'Scriptotar stopped before this job finished. Retry starts a new attempt; it does not pretend to resume the old process.': 'توقف Scriptotar قبل انتهاء هذه المهمة. تبدأ إعادة المحاولة محاولة جديدة ولا تدّعي استكمال العملية السابقة.',
  'Review the failure detail, fix the likely cause, then retry the job.': 'راجع تفاصيل الفشل وأصلح السبب المحتمل ثم أعد محاولة المهمة.',
  'progress not reported by worker': 'العامل لم يبلّغ عن التقدم',
  Queued: 'في الطابور',
  Preparing: 'جارٍ التحضير',
  Downloading: 'جارٍ التنزيل',
  Transcribing: 'جارٍ التفريغ',
  Processing: 'جارٍ المعالجة',
  Completed: 'مكتملة',
  Failed: 'فشلت',
  Cancelled: 'ملغاة',
  Interrupted: 'متوقفة',

  Healthy: 'سليم',
  'Never scanned': 'لم يُفحص بعد',
  Refreshing: 'جارٍ التحديث',
  'Retry scheduled': 'تمت جدولة إعادة المحاولة',
  'Research scan completed.': 'اكتمل فحص البحث.',
  'Watchlist saved locally for this project.': 'تم حفظ قائمة المتابعة محلياً لهذا المشروع.',
  'Selected media queued.': 'تمت إضافة الوسائط المحددة إلى الطابور.',
  'Creator intelligence': 'تحليل صناع المحتوى',
  'Scan public creator profiles, compare performance signals, then queue only the media worth transcribing.': 'افحص ملفات صناع المحتوى العامة وقارن مؤشرات الأداء ثم أضف فقط الوسائط الجديرة بالتفريغ.',
  'Creator / profile URL': 'رابط صانع المحتوى / الملف',
  'Creator profile URL': 'رابط ملف صانع المحتوى',
  Limit: 'الحد',
  'Scanning…': 'جارٍ الفحص…',
  'Scan profile': 'فحص الملف',
  'Saving…': 'جارٍ الحفظ…',
  'Save watchlist': 'حفظ قائمة المتابعة',
  'Background refresh': 'التحديث في الخلفية',
  'Watchlist health': 'حالة قوائم المتابعة',
  'Failures and retry timing are stored locally, including across restarts.': 'تُحفظ حالات الفشل ومواعيد إعادة المحاولة محلياً حتى بعد إعادة التشغيل.',
  'No saved watchlists in this project yet.': 'لا توجد قوائم متابعة محفوظة في هذا المشروع بعد.',
  'Last attempt': 'آخر محاولة',
  'Last success': 'آخر نجاح',
  'Next retry': 'إعادة المحاولة التالية',
  'This creator has not completed a watchlist scan yet.': 'لم يكتمل فحص قائمة المتابعة لهذا الصانع بعد.',
  'A background creator scan is currently running.': 'يجري حالياً فحص صانع محتوى في الخلفية.',
  'Filter research': 'تصفية البحث',
  'Filter title or creator…': 'صفِّ حسب العنوان أو صانع المحتوى…',
  'Platform filter': 'مرشح المنصة',
  Sort: 'الترتيب',
  'Research sort': 'ترتيب البحث',
  Views: 'المشاهدات',
  Likes: 'الإعجابات',
  Comments: 'التعليقات',
  Newest: 'الأحدث',
  'No matching research': 'لا توجد نتائج بحث مطابقة',
  'Change the filters or scan another creator profile.': 'غيّر المرشحات أو افحص ملف صانع محتوى آخر.',
  'Research results': 'نتائج البحث',
  Media: 'الوسائط',
  Date: 'التاريخ',
  Duration: 'المدة',

  'Prompt-only mode keeps the generated prompt local.': 'وضع المطالبة فقط يبقي المطالبة المُنشأة محلياً.',
  'Prompt built locally. Nothing has been sent to an AI provider.': 'تم إنشاء المطالبة محلياً. لم يُرسل شيء إلى مزود ذكاء اصطناعي.',
  'Clipboard access is unavailable in this runtime. Select the text and copy it manually.': 'الوصول إلى الحافظة غير متاح في هذا التشغيل. حدد النص وانسخه يدوياً.',
  'Copy Prompt mode is ready for manual use elsewhere.': 'وضع نسخ المطالبة جاهز للاستخدام اليدوي في مكان آخر.',
  'Enter an API key for this session, or switch to Copy Prompt.': 'أدخل مفتاح API لهذه الجلسة أو انتقل إلى وضع نسخ المطالبة.',
  'Unknown error': 'خطأ غير معروف',
  'Optional AI layer': 'طبقة ذكاء اصطناعي اختيارية',
  'Build portable prompts with no key, or use your own provider credentials for a direct run.': 'أنشئ مطالبات قابلة للنقل دون مفتاح أو استخدم بيانات مزودك لتشغيل مباشر.',
  'AI mode': 'وضع الذكاء الاصطناعي',
  'No API key · nothing sent': 'بدون مفتاح API · لا يتم إرسال شيء',
  BYOK: 'مفتاحك الخاص',
  'Use your own provider key': 'استخدم مفتاح مزودك الخاص',
  Task: 'المهمة',
  'Viral breakdown': 'تحليل الانتشار',
  'Hook ideas': 'أفكار افتتاحيات',
  'New short-form script': 'نص قصير جديد',
  'Structure remix': 'إعادة تركيب البنية',
  'Content ideas': 'أفكار محتوى',
  'Caption + CTA': 'وصف + دعوة لاتخاذ إجراء',
  'Voice profile': 'ملف أسلوب الصوت',
  'B-roll shot list': 'قائمة لقطات B-roll',
  Provider: 'المزود',
  'AI provider': 'مزود الذكاء الاصطناعي',
  'OpenAI-compatible': 'متوافق مع OpenAI',
  'Local (coming later)': 'محلي (قريباً)',
  Model: 'النموذج',
  'API key · session only': 'مفتاح API · لهذه الجلسة فقط',
  'API key': 'مفتاح API',
  'Paste key for this run': 'ألصق المفتاح لهذا التشغيل',
  'Base URL': 'الرابط الأساسي',
  'Keys are not persisted by this frontend. The Rust host will own secure storage and endpoint policy.': 'لا تحفظ الواجهة هذه المفاتيح. يتولى مضيف Rust التخزين الآمن وسياسة نقاط الاتصال.',
  Input: 'الإدخال',
  'Source context': 'سياق المصدر',
  spoken: 'منطوق',
  'Paste or load transcript/research text…': 'ألصق أو حمّل نص تفريغ/بحث…',
  'Portable artifact': 'مخرج قابل للنقل',
  'Generated prompt': 'المطالبة المُنشأة',
  'Build a prompt to preview it here…': 'أنشئ مطالبة لمعاينتها هنا…',
  'Topic / goal': 'الموضوع / الهدف',
  Audience: 'الجمهور',
  'Target duration': 'المدة المستهدفة',
  CTA: 'الدعوة لاتخاذ إجراء',
  'Voice / style instructions': 'تعليمات الأسلوب / النبرة',
  'Build prompt': 'إنشاء المطالبة',
  'Copy prompt': 'نسخ المطالبة',
  'Running…': 'جارٍ التشغيل…',
  'Prepare for copy': 'تجهيز للنسخ',
  'Run with API': 'تشغيل عبر API',
  'AI result': 'نتيجة الذكاء الاصطناعي',
  'Copy result': 'نسخ النتيجة',

  'Unified local index': 'فهرس محلي موحد',
  'Browse the active project\'s transcripts, creator research, AI runs, projects, and creators, then open the related workspace directly.': 'تصفح نصوص المشروع النشط وبحوث صناع المحتوى وتشغيلات الذكاء الاصطناعي والمشاريع وصناع المحتوى، ثم افتح مساحة العمل المرتبطة مباشرة.',
  'Search library': 'البحث في المكتبة',
  'Search your local library…': 'ابحث في مكتبتك المحلية…',
  'Library kind': 'نوع المكتبة',
  'AI run': 'تشغيل ذكاء اصطناعي',
  Creator: 'صانع محتوى',
  'Library sort': 'ترتيب المكتبة',
  'Newest first': 'الأحدث أولاً',
  Title: 'العنوان',
  Type: 'النوع',
  item: 'عنصر',
  items: 'عناصر',
  'Nothing matches': 'لا توجد نتائج مطابقة',
  'Try a broader search or another library type.': 'جرّب بحثاً أوسع أو نوع مكتبة آخر.',
  Local: 'محلي',
  'Could not open this library item.': 'تعذر فتح عنصر المكتبة هذا.',

  'Review + reuse': 'مراجعة وإعادة استخدام',
  'Search timestamped text, jump between matching segments, copy clean text, and export local formats without inventing artifact paths.': 'ابحث في النص الموقّت وانتقل بين المقاطع المطابقة وانسخ النص النظيف وصدّر الصيغ محلياً.',
  'No transcripts yet': 'لا توجد نصوص مفرغة بعد',
  'Complete a transcription job and it will appear here.': 'أكمل مهمة تفريغ وستظهر هنا.',
  'Transcript list': 'قائمة النصوص المفرغة',
  'Copy text': 'نسخ النص',
  'Export TXT': 'تصدير TXT',
  'Search transcript': 'البحث في النص المفرغ',
  'Search transcript…': 'ابحث في النص المفرغ…',
  segment: 'مقطع',
  segments: 'مقاطع',
  Details: 'التفاصيل',
  'Source metadata': 'بيانات المصدر',
  Direction: 'الاتجاه',
  Platform: 'المنصة',
  Exports: 'التصدير',
  'Timestamp TXT': 'TXT مع التوقيت',
  'Open output folder unavailable': 'فتح مجلد الإخراج غير متاح',
  'The backend does not expose a persisted artifact directory for this transcript yet.': 'الخلفية لا توفر حتى الآن مجلد مخرجات محفوظاً لهذا النص المفرغ.',
  'Transcript copied to the clipboard.': 'تم نسخ النص المفرغ إلى الحافظة.',
  'Clipboard access is unavailable in this desktop runtime.': 'الوصول إلى الحافظة غير متاح في تشغيل سطح المكتب هذا.',
  'Could not copy the transcript.': 'تعذر نسخ النص المفرغ.',

  'Local preferences': 'التفضيلات المحلية',
  'Transcription, downloads, storage, privacy, and appearance stay explicit. Settings are validated below the UI before they are persisted.': 'تبقى إعدادات التفريغ والتنزيل والتخزين والخصوصية والمظهر واضحة، ويتم التحقق منها قبل حفظها.',
  'Save changes': 'حفظ التغييرات',
  Transcription: 'التفريغ',
  'Speech engine': 'محرك الكلام',
  'Choose quality and compute defaults. Model installation remains a host responsibility.': 'اختر إعدادات الجودة والحوسبة الافتراضية. يبقى تثبيت النماذج مسؤولية المضيف.',
  'Whisper model': 'نموذج Whisper',
  Device: 'الجهاز',
  auto: 'تلقائي',
  'Downloads + cookies': 'التنزيلات + ملفات تعريف الارتباط',
  'Media acquisition': 'جلب الوسائط',
  'Browser cookies are selected by browser name only. Cookie secrets never belong in the frontend.': 'يتم اختيار ملفات تعريف الارتباط باسم المتصفح فقط، ولا تُحفظ الأسرار في الواجهة.',
  Quality: 'الجودة',
  Best: 'الأفضل',
  'Audio only': 'صوت فقط',
  'Browser cookies': 'ملفات تعريف ارتباط المتصفح',
  none: 'بدون',
  'Duration safety limit': 'حد أمان المدة',
  '30 min': '30 دقيقة',
  '60 min': '60 دقيقة',
  '2 hours': 'ساعتان',
  '6 hours': '6 ساعات',
  Unlimited: 'غير محدود',
  Storage: 'التخزين',
  'Transcript output': 'مخرجات التفريغ',
  'Choose where new transcription result folders are created. Rust verifies that a selected directory exists and is writable before saving it.': 'اختر مكان إنشاء مجلدات نتائج التفريغ الجديدة. يتحقق Rust من وجود المجلد وإمكانية الكتابة فيه قبل حفظه.',
  'Current output directory': 'مجلد الإخراج الحالي',
  'Application default': 'الإعداد الافتراضي للتطبيق',
  'Choose output folder': 'اختيار مجلد الإخراج',
  'Restore default': 'استعادة الافتراضي',
  'Privacy + processing': 'الخصوصية + المعالجة',
  'Local behavior': 'السلوك المحلي',
  'These values are persisted by the Rust settings layer and applied to future jobs.': 'تُحفظ هذه القيم في طبقة إعدادات Rust وتُطبق على المهام المستقبلية.',
  'Copy local source media': 'نسخ الوسائط المصدر المحلية',
  'Keep a copy beside generated transcript artifacts.': 'احتفظ بنسخة بجانب ملفات النص المفرغ المُنشأة.',
  'Translate speech to English': 'ترجمة الكلام إلى الإنجليزية',
  'Ask the transcription worker to translate.': 'اطلب من عامل التفريغ إجراء الترجمة.',
  'Batched inference': 'استدلال على دفعات',
  'Faster on suitable hardware, with higher memory use.': 'أسرع على العتاد المناسب مع استهلاك ذاكرة أكبر.',
  'Keep failed partial artifacts': 'الاحتفاظ بالمخرجات الجزئية الفاشلة',
  'Useful for debugging interrupted media stages.': 'مفيد لتشخيص مراحل الوسائط المتوقفة.',
  'Creator watchlists': 'قوائم متابعة صناع المحتوى',
  'Automatic refresh uses the configured local research provider. Failures, retry timing, and recovery are visible in Research.': 'يستخدم التحديث التلقائي مزود البحث المحلي المضبوط، وتظهر حالات الفشل وإعادة المحاولة والاسترداد في البحث.',
  'Refresh saved watchlists automatically': 'تحديث قوائم المتابعة المحفوظة تلقائياً',
  'Scriptotar scans due watchlists while the app is running and records failures instead of hiding them.': 'يفحص Scriptotar قوائم المتابعة المستحقة أثناء تشغيل التطبيق ويسجل حالات الفشل بدلاً من إخفائها.',
  'Refresh interval': 'فترة التحديث',
  'Creator watch refresh': 'تحديث متابعة صناع المحتوى',
  'Legacy migration': 'ترحيل النسخة القديمة',
  'Import Scriptotar Classic data': 'استيراد بيانات Scriptotar Classic',
  'Discovery uses a read-only, WAL-aware SQLite snapshot. Source databases are never selected by raw frontend paths and are not overwritten.': 'يستخدم الاكتشاف لقطة SQLite للقراءة فقط ومدركة لـ WAL. لا تختار الواجهة قواعد المصدر بمسارات خام ولا تتم الكتابة فوقها.',
  'No legacy database found': 'لم يتم العثور على قاعدة بيانات قديمة',
  'Ready to import': 'جاهز للاستيراد',
  Importing: 'جارٍ الاستيراد',
  'Choice required': 'يلزم الاختيار',
  'Invalid legacy database': 'قاعدة البيانات القديمة غير صالحة',
  'Migration failed': 'فشل الترحيل',
  'Scriptotar is importing the prepared legacy snapshot. The source database remains untouched.': 'يقوم Scriptotar باستيراد اللقطة القديمة المُجهزة. تبقى قاعدة البيانات المصدر دون تعديل.',
  'The migration request could not be completed. The legacy source database was not modified; retry when the local error is resolved.': 'تعذر إكمال طلب الترحيل. لم يتم تعديل قاعدة البيانات القديمة؛ أعد المحاولة بعد حل الخطأ المحلي.',
  'Legacy database choices': 'خيارات قاعدة البيانات القديمة',
  'Checking…': 'جارٍ التحقق…',
  'Import prepared snapshot': 'استيراد اللقطة المُجهزة',
  'Retry migration discovery': 'إعادة اكتشاف الترحيل',
  Appearance: 'المظهر',
  Interface: 'الواجهة',
  'Dark stays fixed; System follows the operating-system light/dark preference. Appearance is a local UI preference and contains no sensitive data.': 'يبقى الوضع الداكن ثابتاً، بينما يتبع وضع النظام تفضيل نظام التشغيل. المظهر تفضيل محلي للواجهة ولا يحتوي بيانات حساسة.',
  Theme: 'السمة',
  Dark: 'داكن',
  System: 'النظام',
  'Settings saved. New transcription jobs and watchlist refreshes will use these preferences.': 'تم حفظ الإعدادات. ستستخدم مهام التفريغ وتحديثات قوائم المتابعة الجديدة هذه التفضيلات.',
  'Could not save settings.': 'تعذر حفظ الإعدادات.',
  'Output folder selected. Save changes to make it the default.': 'تم اختيار مجلد الإخراج. احفظ التغييرات لجعله الافتراضي.',
  'Could not choose an output folder.': 'تعذر اختيار مجلد إخراج.',
  'Default output location selected. Save changes to apply it.': 'تم اختيار موقع الإخراج الافتراضي. احفظ التغييرات لتطبيقه.'
};

const skipTextSelector = [
  '[data-i18n-ignore]',
  '.transcript-content',
  '.result-copy',
  '.job-title-row strong',
  '.job-meta',
  '.job-detail',
  '.library-list .library-copy',
  '.research-row:not(.table-head) .media-cell',
  '.creator-row strong',
  '.creator-row small',
  '.simple-list strong',
  '.simple-list small',
  '.search-results strong',
  '.search-results small',
  '.project-control option',
  '.transcript-list strong',
  '.transcript-list small',
  '.reader-head h2',
  '.reader-head p',
  '.reader-head .eyebrow',
  '.details-panel dd',
  '.watch-health-title-row strong',
  '.watch-error'
].join(',');

const originalText = new WeakMap<Text, string>();
const originalAttributes = new WeakMap<Element, Map<string, string>>();
const originalOptionValues = new WeakMap<HTMLOptionElement, string>();
let currentLocale: AppLocale = readStoredLocale();
let observer: MutationObserver | null = null;
let initialized = false;

function translatePattern(value: string): string | null {
  let match = value.match(/^(\d+) active jobs$/);
  if (match) return `${match[1]} مهام نشطة`;
  match = value.match(/^(\d+) active$/);
  if (match) return `${match[1]} نشطة`;
  match = value.match(/^(\d+) watchlisted$/);
  if (match) return `${match[1]} ضمن قائمة المتابعة`;
  match = value.match(/^(\d+) items?$/);
  if (match) return `${match[1]} عنصر`;
  match = value.match(/^(\d+) matching segments?$/);
  if (match) return `${match[1]} مقطع مطابق`;
  match = value.match(/^Queue selected \((\d+)\)$/);
  if (match) return `إضافة المحدد للطابور (${match[1]})`;
  match = value.match(/^Queued (.+)\.$/);
  if (match) return `تمت إضافة ${match[1]} إلى الطابور.`;
  match = value.match(/^(.+) export prepared\.$/);
  if (match) return `تم تجهيز تصدير ${match[1]}.`;
  match = value.match(/^Finished with (.+)\.$/);
  if (match) return `اكتمل باستخدام ${match[1]}.`;
  match = value.match(/^(.+) copied to the clipboard\.$/);
  if (match) return `تم نسخ ${arabic[match[1]] ?? match[1]} إلى الحافظة.`;
  match = value.match(/^Copy failed: (.+)$/);
  if (match) return `فشل النسخ: ${match[1]}`;
  match = value.match(/^AI request failed: (.+)$/);
  if (match) return `فشل طلب الذكاء الاصطناعي: ${match[1]}`;
  match = value.match(/^Research scan unavailable: (.+)$/);
  if (match) return `فحص البحث غير متاح: ${match[1]}`;
  match = value.match(/^Watchlist save failed: (.+)$/);
  if (match) return `فشل حفظ قائمة المتابعة: ${match[1]}`;
  match = value.match(/^Queue unavailable: (.+)$/);
  if (match) return `الطابور غير متاح: ${match[1]}`;
  match = value.match(/^No timestamped segment contains “(.+)”\.$/);
  if (match) return `لا يوجد مقطع موقّت يحتوي على “${match[1]}”.`;
  match = value.match(/^Select (.+)$/);
  if (match) return `تحديد ${match[1]}`;
  match = value.match(/^Open (Transcript|Research|AI run|Project|Creator): (.+)$/);
  if (match) return `فتح ${arabic[match[1]] ?? match[1]}: ${match[2]}`;
  match = value.match(/^Jump to (.+)$/);
  if (match) return `الانتقال إلى ${match[1]}`;
  match = value.match(/^(\d+) sec$/);
  if (match) return `${match[1]} ث`;
  match = value.match(/^(\d+)s$/);
  if (match) return `${match[1]} ث`;
  match = value.match(/^Imported (\d+) projects, (\d+) jobs, (\d+) transcripts, (\d+) research items, (\d+) watchlists, and (\d+) AI runs\.$/);
  if (match) return `تم استيراد ${match[1]} مشروع، و${match[2]} مهمة، و${match[3]} نصوص مفرغة، و${match[4]} عناصر بحث، و${match[5]} قوائم متابعة، و${match[6]} تشغيلات ذكاء اصطناعي.`;
  return null;
}

export function translateUiText(source: string): string {
  const leading = source.match(/^\s*/)?.[0] ?? '';
  const trailing = source.match(/\s*$/)?.[0] ?? '';
  const end = trailing.length ? source.length - trailing.length : source.length;
  const core = source.slice(leading.length, end);
  if (!core) return source;
  const translated = arabic[core] ?? translatePattern(core);
  return translated ? `${leading}${translated}${trailing}` : source;
}

function applyDocumentMetadata() {
  if (typeof document === 'undefined') return;
  document.documentElement.lang = currentLocale;
  document.documentElement.dir = currentLocale === 'ar' ? 'rtl' : 'ltr';
  document.documentElement.dataset.locale = currentLocale;
}

function shouldSkipText(node: Text): boolean {
  const parent = node.parentElement;
  return Boolean(parent?.closest(skipTextSelector));
}

function renderText(node: Text, captureOriginal = false) {
  if (shouldSkipText(node)) return;
  const parent = node.parentElement;
  if (currentLocale === 'ar') {
    if (parent instanceof HTMLOptionElement && !parent.hasAttribute('value') && !originalOptionValues.has(parent)) {
      const originalValue = parent.value;
      originalOptionValues.set(parent, originalValue);
      parent.value = originalValue;
    }
    if (captureOriginal || !originalText.has(node)) originalText.set(node, node.data);
    const source = originalText.get(node) ?? node.data;
    node.data = translateUiText(source);
  } else {
    const source = originalText.get(node);
    if (source !== undefined) node.data = source;
  }
}

function attributeMap(element: Element): Map<string, string> {
  let values = originalAttributes.get(element);
  if (!values) {
    values = new Map<string, string>();
    originalAttributes.set(element, values);
  }
  return values;
}

function renderAttribute(element: Element, name: string, captureOriginal = false) {
  const current = element.getAttribute(name);
  if (current === null || element.closest('[data-i18n-ignore]')) return;
  const values = attributeMap(element);
  if (currentLocale === 'ar') {
    if (captureOriginal || !values.has(name)) values.set(name, current);
    const source = values.get(name) ?? current;
    element.setAttribute(name, translateUiText(source));
  } else {
    const source = values.get(name);
    if (source !== undefined) element.setAttribute(name, source);
  }
}

function renderSubtree(root: Node, captureOriginal = false) {
  if (root.nodeType === Node.TEXT_NODE) renderText(root as Text, captureOriginal);

  if (root instanceof Element) {
    for (const attribute of TRANSLATED_ATTRIBUTES) renderAttribute(root, attribute, captureOriginal);
    const elements = root.querySelectorAll('*');
    for (const element of elements) {
      for (const attribute of TRANSLATED_ATTRIBUTES) renderAttribute(element, attribute, captureOriginal);
    }
  }

  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let next = walker.nextNode();
  while (next) {
    renderText(next as Text, captureOriginal);
    next = walker.nextNode();
  }
}

function observe() {
  if (!observer || !document.body) return;
  observer.observe(document.body, {
    subtree: true,
    childList: true,
    characterData: true,
    attributes: true,
    attributeFilter: [...TRANSLATED_ATTRIBUTES]
  });
}

function withObserverPaused(action: () => void) {
  observer?.disconnect();
  action();
  observe();
}

function renderDocument() {
  if (!initialized || typeof document === 'undefined' || !document.body) return;
  applyDocumentMetadata();
  withObserverPaused(() => renderSubtree(document.body));
}

export function initializeLocalization() {
  if (initialized || typeof document === 'undefined' || !document.body) return;
  initialized = true;
  observer = new MutationObserver((mutations) => {
    withObserverPaused(() => {
      for (const mutation of mutations) {
        if (mutation.type === 'characterData') {
          renderText(mutation.target as Text, currentLocale === 'ar');
        } else if (mutation.type === 'attributes' && mutation.target instanceof Element && mutation.attributeName) {
          renderAttribute(mutation.target, mutation.attributeName, currentLocale === 'ar');
        } else if (mutation.type === 'childList') {
          for (const node of mutation.addedNodes) renderSubtree(node);
        }
      }
    });
  });
  renderDocument();
  observe();
}

export function setLocale(next: AppLocale) {
  if (next !== 'en' && next !== 'ar') return;
  try {
    window.localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // Local persistence can be unavailable in hardened webviews; the in-memory setting still works.
  }
  locale.set(next);
}

export function getLocale(): AppLocale {
  return get(locale);
}

locale.subscribe((next) => {
  currentLocale = next;
  applyDocumentMetadata();
  renderDocument();
});
