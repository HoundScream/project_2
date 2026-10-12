use crate::content::{
    metadata::PageMetadata,
    schema::{
        ActionLink, ActionVariant, CtaContent, FaqItem, FeatureItem, FooterContent, HeroContent,
        PageContent, SearchFormContent,
    },
    seo::SeoContent,
};

pub const PAGE: PageContent = PageContent {
    metadata: PageMetadata {
        id: "landing",
        route: "/",
        locale: "ru",
        cms_collection: "pages",
    },
    seo: SeoContent {
        title: "Geonetix",
        description: "Geonetix помогает анализировать карточки бизнеса, конкурентов и качество присутствия на картах.",
        canonical_path: "/",
        robots: "index,follow",
        keywords: &[
            "Geonetix",
            "анализ карточек бизнеса",
            "локальное SEO",
            "Яндекс Карты",
            "Google Maps",
            "2ГИС",
        ],
    },
};

pub const HERO: HeroContent = HeroContent {
    eyebrow: None,
    title: "Geonetix для анализа карточек бизнеса на картах",
    description: "Приложение готовится для анализа конкурентов и качества карточек компаний в Яндекс Картах, Google Maps и 2ГИС. Уже сейчас можно пройти демо-поиск и открыть путь к бесплатному кабинету.",
};

pub const HERO_FORM: SearchFormContent = SearchFormContent {
    action: "/search",
    method: "get",
    field_name: "q",
    input_type: "search",
    placeholder: "Введите категорию или город...",
    aria_label: "Поисковый запрос",
    submit_label: "Попробовать демо-анализ",
};

pub const HERO_ACTIONS: &[ActionLink] = &[
    ActionLink {
        label: "Регистрация пользователя",
        href: "/register",
        variant: ActionVariant::Primary,
    },
    ActionLink {
        label: "Открыть пример поиска",
        href: "/search?q=%D0%BC%D0%BE%D1%81%D0%BA%D0%B2%D0%B0",
        variant: ActionVariant::Secondary,
    },
];

pub const FEATURES_TITLE: &str = "Что уже заложено в продукт";

pub const FEATURES: &[FeatureItem] = &[
    FeatureItem {
        title: None,
        description: "Отдельные адреса для экранов: лендинг, поиск, регистрация и страницы следующих модулей.",
    },
    FeatureItem {
        title: None,
        description: "Реальная регистрационная модель пользователя с тарифом, статусом и лимитом бесплатного анализа.",
    },
    FeatureItem {
        title: None,
        description: "Подготовка к анализу карточек бизнеса из картографических сервисов с сохранением отчётов в личном кабинете.",
    },
];

pub const FAQ_TITLE: &str = "Частые вопросы";

pub const FAQ: &[FaqItem] = &[
    FaqItem {
        question: "Где редактировать тексты лендинга?",
        answer: "Все тексты лендинга теперь лежат в `content/pages/landing.rs`, а не в HTML-рендерере.",
    },
    FaqItem {
        question: "Подготовлена ли структура под мультиязычность?",
        answer: "Да. Контент отделён по страницам и секциям, поэтому следующий шаг — вынести те же схемы в `content/locales/<locale>/...` или подключить CMS-слой.",
    },
    FaqItem {
        question: "Зачем это удобно для AI-ассистентов?",
        answer: "AI-агенты могут менять copy в маленьких изолированных файлах, не открывая большой `src/main.rs` и не рискуя сломать UI-разметку.",
    },
];

pub const CTA: CtaContent = CtaContent {
    title: "Готовы перейти к сценарию глубже?",
    description: "Начните с демо-поиска или откройте регистрацию. Весь следующий текстовый слой подключается через `content`, а не через UI-функции.",
    primary_action: ActionLink {
        label: "Попробовать демо-анализ",
        href: "/search?q=%D0%BA%D0%BE%D1%84%D0%B5%D0%B9%D0%BD%D1%8F",
        variant: ActionVariant::Primary,
    },
    secondary_action: Some(ActionLink {
        label: "Открыть регистрацию",
        href: "/register",
        variant: ActionVariant::Secondary,
    }),
};

pub const FOOTER: FooterContent = FooterContent {
    note: "Geonetix хранит маркетинговый и интерфейсный copy централизованно, чтобы его можно было безопасно обновлять под CMS, локализацию и AI-редактирование.",
    links: &[ActionLink {
        label: "Перейти к тарифам",
        href: "/pricing",
        variant: ActionVariant::Secondary,
    }],
};
