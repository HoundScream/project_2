use crate::content::{
    metadata::PageMetadata,
    schema::{
        ActionLink, ActionVariant, FeatureItem, FooterContent, FormFieldContent, HeroContent,
        PageContent, SelectFieldContent, SelectOption, StatusMessageContent,
    },
    seo::SeoContent,
};

pub const PAGE: PageContent = PageContent {
    metadata: PageMetadata {
        id: "pro-package",
        route: "/pricing/pro",
        locale: "ru",
        cms_collection: "pages",
    },
    seo: SeoContent {
        title: "Модуль 6.1. Пакет PRO",
        description: "Шаг после оплаты Geonetix PRO: сбор бизнес-запроса, выбор категории и запуск валидации перед мониторингом.",
        canonical_path: "/pricing/pro",
        robots: "noindex,nofollow",
        keywords: &[
            "Geonetix PRO",
            "бизнес-запрос",
            "категория бизнеса",
            "валидатор",
        ],
    },
};

pub const HERO: HeroContent = HeroContent {
    eyebrow: Some("Модуль 6.1. Пакет PRO"),
    title: "Следующий шаг после оплаты",
    description: "Пользователь вводит адрес или ставит точку на карте, затем выбирает категорию бизнеса. После этого система запускает первичную проверку запроса перед полноценным PRO-анализом.",
};

pub const QUERY_FIELD: FormFieldContent = FormFieldContent {
    name: "business_query",
    label: "Введите адрес / поставьте точку на карте",
    input_type: "text",
    placeholder: "Например, Москва, ул. Тверская, 7 или кофейня возле метро Белорусская",
};

pub const CATEGORY_OPTIONS: &[SelectOption] = &[
    SelectOption {
        value: "",
        label: "Выберите категорию бизнеса",
    },
    SelectOption {
        value: "Ресторан / кафе",
        label: "Ресторан / кафе",
    },
    SelectOption {
        value: "Салон красоты",
        label: "Салон красоты",
    },
    SelectOption {
        value: "Медицинская клиника",
        label: "Медицинская клиника",
    },
    SelectOption {
        value: "Фитнес / спорт",
        label: "Фитнес / спорт",
    },
    SelectOption {
        value: "Магазин / ритейл",
        label: "Магазин / ритейл",
    },
    SelectOption {
        value: "Услуги для дома",
        label: "Услуги для дома",
    },
];

pub const CATEGORY_FIELD: SelectFieldContent = SelectFieldContent {
    name: "business_category",
    label: "Выберите категорию бизнеса",
    options: CATEGORY_OPTIONS,
};

pub const SUBMIT_LABEL: &str = "Анализировать";

pub const STATUS: StatusMessageContent = StatusMessageContent {
    title: "Система анализирует запрос на ошибки",
    success_text: "После успешной проверки форма сразу переведёт пользователя в модуль 7: ежедневный PRO-мониторинг и системный журнал изменений.",
    error_text: "Этот блок показывает, что происходит сразу после оплаты: мы сначала проверяем полноту входных данных, чтобы не запускать PRO-анализ в пустоту.",
    next_step_title: "Следующий шаг:",
    next_step_description: "если поля заполнены корректно, создадим сценарий ежедневного мониторинга и откроем журнал уведомлений.",
};

pub const BENEFITS: &[FeatureItem] = &[
    FeatureItem {
        title: Some("Валидация до запуска"),
        description: "Система отсеивает пустые и неполные запросы до создания PRO-сценария.",
    },
    FeatureItem {
        title: Some("Единый источник правды"),
        description: "Copy для шага после оплаты хранится отдельно и может обновляться без изменения HTML-структуры.",
    },
];

pub const FOOTER: FooterContent = FooterContent {
    note: "Форма и служебные пояснения привязаны к reusable schema, чтобы их можно было заменить на CMS-поля без рефакторинга страницы.",
    links: &[ActionLink {
        label: "Вернуться к оплате",
        href: "/pricing",
        variant: ActionVariant::Secondary,
    }],
};
