use crate::content::{
    metadata::PageMetadata,
    schema::{ActionLink, ActionVariant, FooterContent, PageContent},
    seo::SeoContent,
};

pub const PAGE: PageContent = PageContent {
    metadata: PageMetadata {
        id: "not-found",
        route: "/404",
        locale: "ru",
        cms_collection: "system-pages",
    },
    seo: SeoContent {
        title: "Страница не найдена",
        description: "Служебная страница Geonetix для ошибок маршрутов и отсутствующих объектов.",
        canonical_path: "/404",
        robots: "noindex,nofollow",
        keywords: &["404", "ошибка", "Geonetix"],
    },
};

pub const HOME_LINK: ActionLink = ActionLink {
    label: "Вернуться на главную",
    href: "/",
    variant: ActionVariant::Secondary,
};

pub const FOOTER: FooterContent = FooterContent {
    note: "Даже системные страницы используют общий content-layer, чтобы AI-агенты меняли сообщения точечно и без копипаста.",
    links: &[HOME_LINK],
};
