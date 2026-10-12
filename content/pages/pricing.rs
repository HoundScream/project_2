use crate::content::{
    metadata::PageMetadata,
    schema::{
        ActionLink, ActionVariant, CtaContent, FeatureItem, FooterContent, HeroContent, PageContent,
    },
    seo::SeoContent,
};

pub const PAGE: PageContent = PageContent {
    metadata: PageMetadata {
        id: "pricing",
        route: "/pricing",
        locale: "ru",
        cms_collection: "pages",
    },
    seo: SeoContent {
        title: "Geonetix PRO",
        description: "Страница оплаты Geonetix PRO с описанием ценности пакета, этапов сценария и перехода к мониторингу.",
        canonical_path: "/pricing",
        robots: "noindex,nofollow",
        keywords: &[
            "Geonetix PRO",
            "оплата",
            "мониторинг конкурентов",
            "локальное SEO",
        ],
    },
};

pub const HERO: HeroContent = HeroContent {
    eyebrow: Some("Geonetix PRO"),
    title: "Модуль 6. Оплата",
    description: "Здесь пользователь видит простую причину оплаты: бесплатный анализ уже показал ценность, а PRO открывает следующий объём работы без перегруза лишними шагами.",
};

pub const PRICE_LABEL: &str = "Пакет PRO: 3 запроса + анализ 5 конкурентов";

pub const FEATURES: &[FeatureItem] = &[
    FeatureItem {
        title: None,
        description: "3 новых запроса анализа вместо бесплатного лимита.",
    },
    FeatureItem {
        title: None,
        description: "Сравнение с 5 конкурентами на этапе `compare_competitors`.",
    },
    FeatureItem {
        title: None,
        description: "Понятный переход к следующему действию после оплаты без возврата в кабинет.",
    },
];

pub const CTA: CtaContent = CtaContent {
    title: "Следующий шаг",
    description: "Переход ведёт в модуль 6.1, где пользователь задаёт точку анализа и категорию бизнеса.",
    primary_action: ActionLink {
        label: "Оплатить PRO",
        href: "/pricing/pro",
        variant: ActionVariant::Primary,
    },
    secondary_action: None,
};

pub const FLOW_TITLE: &str = "Структура оплаты";

pub const FLOW: &[FeatureItem] = &[
    FeatureItem {
        title: Some("Модуль 5"),
        description: "Пользователь получает обработанный бесплатный анализ, видит пользу и кнопку перехода к полному сценарию.",
    },
    FeatureItem {
        title: Some("Модуль 6"),
        description: "Экран оплаты объясняет, что входит в PRO и почему это следующий логичный шаг.",
    },
    FeatureItem {
        title: Some("Модуль 6.1"),
        description: "После оплаты открывается пакет PRO: ввод адреса или точки, выбор категории бизнеса и запуск проверки запроса.",
    },
    FeatureItem {
        title: Some("Модуль 7"),
        description: "После успешной проверки включается ежедневный мониторинг конкурентов и формируется журнал изменений в кабинете.",
    },
];

pub const FOOTER: FooterContent = FooterContent {
    note: "SEO, metadata и видимый copy для pricing-page разделены, чтобы менять коммерческий текст без пересборки UI-структуры.",
    links: &[ActionLink {
        label: "Вернуться на главную",
        href: "/",
        variant: ActionVariant::Secondary,
    }],
};
