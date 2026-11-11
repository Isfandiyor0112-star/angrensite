use hmac::{Hmac, Mac};
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
    Client, Collection, IndexModel,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::env;
use tokio::sync::OnceCell;

type HmacSha256 = Hmac<Sha256>;

static MONGO_CLIENT: OnceCell<Client> = OnceCell::const_new();

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParticipantResult {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub full_name: String,
    pub phone: String,
    pub school: String,
    pub score: i32,          // Maksimal 60 ball (30 * 2)
    pub correct_count: i32,  // 30 tadan nechta topdi
    pub time_ms: i64,        // Millisekundlarda sarflangan aniq vaqt
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionPayload {
    pub full_name: String,
    pub phone: String,
    pub school: String,
    pub started_at_ms: i64,
}

#[derive(Debug, Serialize)]
pub struct PublicQuestion {
    pub id: usize,
    pub q_uz: &'static str,
    pub q_ru: &'static str,
    pub options_uz: [&'static str; 4],
    pub options_ru: [&'static str; 4],
}

pub struct InternalQuestion {
    pub id: usize,
    pub q_uz: &'static str,
    pub q_ru: &'static str,
    pub options_uz: [&'static str; 4],
    pub options_ru: [&'static str; 4],
    pub correct_index: usize, // Faqat serverda saqlanadi (0, 1, 2 yoki 3)
}

pub const QUESTIONS: [InternalQuestion; 30] = [
    InternalQuestion {
        id: 1,
        q_uz: "O‘zbekiston Respublikasining «Davlat bayrog‘i to‘g‘risida»gi Qonuni qachon qabul qilingan?",
        q_ru: "Когда был принят Закон «О Государственном флаге Республики Узбекистан»?",
        options_uz: ["1991-yil 1-sentabr", "1991-yil 18-noyabr", "1992-yil 2-iyul", "1992-yil 8-dekabr"],
        options_ru: ["1 сентября 1991 года", "18 ноября 1991 года", "2 июля 1992 года", "8 декабря 1992 года"],
        correct_index: 1,
    },
    InternalQuestion {
        id: 2,
        q_uz: "Davlat bayrog‘i O‘zbekiston Oliy Kengashining nechanchi sessiyasida tasdiqlangan?",
        q_ru: "На какой сессии Верховного Совета Узбекистана был утверждён Государственный флаг?",
        options_uz: ["Navbatdan tashqari VII sessiyasida", "VI sessiyasida", "VIII sessiyasida", "X sessiyasida"],
        options_ru: ["На внеочередной VII сессии", "На VI сессии", "На VIII сессии", "На X сессии"],
        correct_index: 0,
    },
    InternalQuestion {
        id: 3,
        q_uz: "Davlat bayrog‘ining rasmiy uzunligi va eni (kengligi) o‘lchamlari qanday?",
        q_ru: "Каковы официальные размеры длины и ширины Государственного флага?",
        options_uz: ["200 sm x 100 sm", "180 sm x 90 sm", "250 sm x 125 sm", "300 sm x 150 sm"],
        options_ru: ["200 см x 100 см", "180 см x 90 см", "250 см x 125 см", "300 см x 150 см"],
        correct_index: 2,
    },
    InternalQuestion {
        id: 4,
        q_uz: "Bayroqdagi uchta asosiy gorizontal yo‘l (moviy, oq, yashil)ning har birining eni necha santimetr?",
        q_ru: "Какова ширина каждой из трёх основных горизонтальных полос (голубой, белой, зелёной) на флаге?",
        options_uz: ["35 sm", "40 sm", "42 sm", "45 sm"],
        options_ru: ["35 см", "40 см", "42 см", "45 см"],
        correct_index: 1,
    },
    InternalQuestion {
        id: 5,
        q_uz: "Oq yo‘lning chetlaridagi qizil hoshiya chiziqlarning har birining eni necha santimetrga teng?",
        q_ru: "Какова ширина каждой из красных каёмок по краям белой полосы?",
        options_uz: ["5 sm", "1,5 sm", "3 sm", "2,5 sm"],
        options_ru: ["5 см", "1,5 см", "3 см", "2,5 см"],
        correct_index: 3,
    },
    InternalQuestion {
        id: 6,
        q_uz: "Bayroqdagi 12 ta oq yulduz necha qatorga va qanday tartibda joylashtirilgan?",
        q_ru: "В сколько рядов и в каком порядке расположены 12 белых звёзд на флаге?",
        options_uz: [
            "3 qator: yuqorida 3, o‘rtada 4, pastda 5 ta",
            "3 qator: har qatorda 4 tadan",
            "2 qator: har qatorda 6 tadan",
            "3 qator: yuqorida 5, o‘rtada 4, pastda 3 ta"
        ],
        options_ru: [
            "В 3 ряда: в верхнем 3, среднем 4, нижнем 5",
            "В 3 ряда: по 4 звезды в каждом",
            "В 2 ряда: по 6 звёзд в каждом",
            "В 3 ряда: в верхнем 5, среднем 4, нижнем 3"
        ],
        correct_index: 0,
    },
    InternalQuestion {
        id: 7,
        q_uz: "Bayroqdagi har bir besh qirrali yulduz diametri necha santimetr bo‘lgan aylanaga ichki chizilgan?",
        q_ru: "В окружность какого диаметра вписана каждая пятиконечная звезда на флаге?",
        options_uz: ["5 sm", "6 sm", "8 sm", "10 sm"],
        options_ru: ["5 см", "6 см", "8 см", "10 см"],
        correct_index: 1,
    },
    InternalQuestion {
        id: 8,
        q_uz: "Yangi oy tasviri diametri necha santimetr bo‘lgan ikki aylananing kesishishidan hosil bo‘ladi?",
        q_ru: "Пересечением двух окружностей какого диаметра образуется изображение полумесяца?",
        options_uz: ["25 sm va 20 sm", "35 sm va 28 sm", "30 sm va 24 sm", "40 sm va 30 sm"],
        options_ru: ["25 см и 20 см", "35 см и 28 см", "30 см и 24 см", "40 см и 30 см"],
        correct_index: 2,
    },
    InternalQuestion {
        id: 9,
        q_uz: "Yangi oy va yulduzlar to‘plami joylashgan to‘g‘ri to‘rtburchakning o‘lchami qanday?",
        q_ru: "Каков размер прямоугольника, в который вписаны полумесяц и звёзды?",
        options_uz: ["45 x 30 sm", "30 x 25 sm", "50 x 35 sm", "40 x 20 sm"],
        options_ru: ["45 x 30 см", "30 x 25 см", "50 x 35 см", "40 x 20 см"],
        correct_index: 1,
    },
    InternalQuestion {
        id: 10,
        q_uz: "Yangi oyning dastaga yaqin nuqtasi bayroq dastasidan necha santimetr masofada joylashgan?",
        q_ru: "На каком расстоянии от древка находится ближайшая к нему точка полумесяца?",
        options_uz: ["15 sm", "25 sm", "10 sm", "20 sm"],
        options_ru: ["15 см", "25 см", "10 см", "20 см"],
        correct_index: 3,
    },
    InternalQuestion {
        id: 11,
        q_uz: "Qatorlardagi yulduzlar orasidagi (gorizontal va vertikal) masofa necha santimetrni tashkil etadi?",
        q_ru: "Каково расстояние (по горизонтали и вертикали) между звёздами в рядах?",
        options_uz: ["6 sm", "4 sm", "5 sm", "8 sm"],
        options_ru: ["6 см", "4 см", "5 см", "8 см"],
        correct_index: 0,
    },
    InternalQuestion {
        id: 12,
        q_uz: "Bayroqdagi moviy (havorang) yo‘l qaysi tarixiy davlat bayrog‘i rangi bilan hamohangdir?",
        q_ru: "С цветом знамени какого исторического государства перекликается голубая полоса флага?",
        options_uz: ["Xorazmshohlar davlati", "Amir Temur saltanati", "Qoraxoniylar davlati", "Somoniylar davlati"],
        options_ru: ["Государство Хорезмшахов", "Государство Амира Темура", "Государство Караханидов", "Государство Саманидов"],
        correct_index: 1,
    },
    InternalQuestion {
        id: 13,
        q_uz: "Bayroqdagi qizil hoshiya chiziqlar nimani anglatadi?",
        q_ru: "Что символизируют красные каёмки на флаге?",
        options_uz: [
            "Quyosh botishi va tong otishini",
            "Tog‘lar va cho‘llar chegarasini",
            "Vujudimizda jo‘shib oqayotgan hayotiy qudrat irmoqlarini",
            "Sanoat va texnika rivojini"
        ],
        options_ru: [
            "Восход и закат солнца",
            "Границу гор и пустынь",
            "Жизненную силу, пульсирующую в каждом живом существе",
            "Промышленное и техническое развитие"
        ],
        correct_index: 2,
    },
    InternalQuestion {
        id: 14,
        q_uz: "O‘zbekiston bayrog‘ida 12 ta yulduz tasvirlanishi nimaning ramzi hisoblanadi?",
        q_ru: "Символом чего являются 12 звёзд на Государственном флаге Узбекистана?",
        options_uz: [
            "Qadimiy 12 burjli quyosh yilnomasi, komillik va saodat timsoli",
            "12 ta daryo va ko‘llar soni",
            "12 ta tarixiy darvoza",
            "12 ta tog‘ cho‘qqisi"
        ],
        options_ru: [
            "Древний 12-летний солнечный календарь, символ совершенства",
            "Количество 12 рек и озёр",
            "12 исторических ворот",
            "12 горных вершин"
        ],
        correct_index: 0,
    },
    InternalQuestion {
        id: 15,
        q_uz: "O‘zbekiston Respublikasi Davlat bayrog‘i BMT qarorgohi oldida qachon ilk bor ko‘tarilgan?",
        q_ru: "Когда Государственный флаг Республики Узбекистан был впервые поднят перед штаб-квартирой ООН?",
        options_uz: ["1991-yil 18-noyabr", "1992-yil 8-dekabr", "1993-yil 1-sentabr", "1992-yil 2-mart"],
        options_ru: ["18 ноября 1991 года", "8 декабря 1992 года", "1 сентября 1993 года", "2 марта 1992 года"],
        correct_index: 3,
    },
    InternalQuestion {
        id: 16,
        q_uz: "1998-yilda qaysi o‘zbek kosmonavti Davlat bayrog‘imizni «Mir» orbital stansiyasiga olib chiqqan?",
        q_ru: "Какой узбекский космонавт доставил Государственный флаг на орбитальную станцию «Мир» в 1998 году?",
        options_uz: ["Vladimir Jonibekov", "Solijon Sharipov", "Talgat Musabayev", "Yuriy Malenchenko"],
        options_ru: ["Владимир Джанибеков", "Салижан Шарипов", "Талгат Мусабаев", "Юрий Маленченко"],
        correct_index: 1,
    },
    InternalQuestion {
        id: 17,
        q_uz: "1998-yil 22-mayda qaysi alpinist O‘zbekiston bayrog‘ini Everest (8848 m) cho‘qqisiga ilk bor o‘rnatgan?",
        q_ru: "Какой альпинист впервые водрузил флаг Узбекистана на вершину Эвереста (8848 м) 22 мая 1998 года?",
        options_uz: ["Rustam Radjapov", "Ilyos Tuhvatullin", "Sergey Sokolov", "Andrey Zaikin"],
        options_ru: ["Рустам Раджапов", "Ильяс Тухватуллин", "Сергей Соколов", "Андрей Заикин"],
        correct_index: 0,
    },
    InternalQuestion {
        id: 18,
        q_uz: "Davlat bayrog‘idan fuqarolarning erkin foydalanish huquqini kengaytiruvchi Qonun (O‘RQ-853) qachon imzolangan?",
        q_ru: "Когда был подписан Закон (ЗРУ-853), расширивший право граждан на свободное использование флага?",
        options_uz: ["2020-yil 18-noyabr", "2021-yil 1-sentabr", "2023-yil 7-iyul", "2024-yil 8-dekabr"],
        options_ru: ["18 ноября 2020 года", "1 сентября 2021 года", "7 июля 2023 года", "8 декабря 2024 года"],
        correct_index: 2,
    },
    InternalQuestion {
        id: 19,
        q_uz: "Toshkentdagi «Xalqlar do‘stligi» maydonida o‘rnatilgan bosh bayroq ustunining balandligi necha metr?",
        q_ru: "Какова высота главного флагштока, установленного на площади «Дружбы народов» в Ташкенте?",
        options_uz: ["50 metr", "65 metr", "85 metr", "100 metr"],
        options_ru: ["50 метров", "65 метров", "85 метров", "100 метров"],
        correct_index: 1,
    },
    InternalQuestion {
        id: 20,
        q_uz: "O‘zbekiston Respublikasi Konstitutsiyasining nechanchi moddasida Davlat ramzlari (bayroq, gerb, madhiya) belgilangan?",
        q_ru: "В какой статье Конституции Республики Узбекистан закреплены Государственные символы (флаг, герб, гимн)?",
        options_uz: ["1-modda", "8-modda", "12-modda", "5-modda"],
        options_ru: ["Статья 1", "Статья 8", "Статья 12", "Статья 5"],
        correct_index: 3,
    },
    InternalQuestion {
        id: 21,
        q_uz: "Davlat bayrog‘ining eni (125 sm) uning uzunligiga (250 sm) nisbatan qanday matematik nisbatda?",
        q_ru: "В каком математическом соотношении находится ширина Государственного флага (125 см) к его длине (250 см)?",
        options_uz: ["1:2 nisbatda", "2:3 nisbatda", "3:5 nisbatda", "1:3 nisbatda"],
        options_ru: ["В соотношении 1:2", "В соотношении 2:3", "В соотношении 3:5", "В соотношении 1:3"],
        correct_index: 0,
    },
    InternalQuestion {
        id: 22,
        q_uz: "Yangi oy va yulduzlar joylashgan 30x25 sm to‘rtburchak moviy yo‘lning yuqori va pastki chetidan necha sm masofada joylashgan?",
        q_ru: "На каком расстоянии от верхнего и нижнего краёв голубой полосы расположен прямоугольник (30x25 см) с полумесяцем и звёздами?",
        options_uz: ["10 sm", "5 sm", "7,5 sm", "12,5 sm"],
        options_ru: ["10 см", "5 см", "7,5 см", "12,5 см"],
        correct_index: 1,
    },
    InternalQuestion {
        id: 23,
        q_uz: "Yulduzlarning quyi (uchinchi) qatori yangi oyning pastki chetidan necha santimetr masofada joylashadi?",
        q_ru: "На каком расстоянии от нижнего края полумесяца располагается нижний (третий) ряд звёзд?",
        options_uz: ["2 sm", "5 sm", "3,5 sm", "6 sm"],
        options_ru: ["2 см", "5 см", "3,5 см", "6 см"],
        correct_index: 2,
    },
    InternalQuestion {
        id: 24,
        q_uz: "Bayroqdagi oq rang (oq yo‘l) qanday ma'noni ifodalaydi?",
        q_ru: "Что символизирует белый цвет (белая полоса) на Государственном флаге?",
        options_uz: [
            "Musaffo osmon va obihayotni",
            "Tinchlik, poklik, beg‘uborlik va ichki go‘zallikka intilishni",
            "Tabiatning yangilanishi va navqironlikni",
            "Kuch-qudrat va jasoratni"
        ],
        options_ru: [
            "Чистое небо и живительную воду",
            "Мир, чистоту, непорочность и стремление к внутренней красоте",
            "Обновление природы и молодость",
            "Силу, могущество и отвагу"
        ],
        correct_index: 1,
    },
    InternalQuestion {
        id: 25,
        q_uz: "Bayroqdagi yashil rang (quyi yo‘l) nimaning timsoli hisoblanadi?",
        q_ru: "Олицетворением чего является зелёный цвет (нижняя полоса) на флаге?",
        options_uz: [
            "Tabiatning yangilanishi, navqironlik, umid va shodumonlik",
            "Qadimiy shaharlar me'morchiligi",
            "Tog‘ cho‘qqilaridagi muzliklar",
            "Qadimiy yulduzlar xaritasi"
        ],
        options_ru: [
            "Обновление природы, молодость, надежда и радость",
            "Архитектура древних городов",
            "Ледники на горных вершинах",
            "Древняя звёздная карта"
        ],
        correct_index: 0,
    },
    InternalQuestion {
        id: 26,
        q_uz: "Yangi oyni hosil qiluvchi kichik aylananing (diametri 24 sm) markazi katta aylana (30 sm) markazidan qancha masofaga siljigan?",
        q_ru: "На какое расстояние смещён центр малой окружности (24 см) относительно центра большой окружности (30 см), образующих полумесяц?",
        options_uz: ["3 sm", "4 sm", "5 sm", "6 sm"],
        options_ru: ["3 см", "4 см", "5 см", "6 см"],
        correct_index: 3,
    },
    InternalQuestion {
        id: 27,
        q_uz: "«O‘zbekiston Respublikasining Davlat bayrog‘i to‘g‘risida»gi Qonunning nechanchi moddasida bayroqning tasviri va o‘lchamlari berilgan?",
        q_ru: "В какой статье Закона «О Государственном флаге Республики Узбекистан» приведено описание и размеры флага?",
        options_uz: ["1-modda", "2-modda", "4-modda", "10-modda"],
        options_ru: ["Статья 1", "Статья 2", "Статья 4", "Статья 10"],
        correct_index: 2,
    },
    InternalQuestion {
        id: 28,
        q_uz: "Motam kunlarida Davlat bayrog‘i bayroq dastasining qancha qismiga qadar tushiriladi?",
        q_ru: "На какую часть длины древка приспускается Государственный флаг в дни траура?",
        options_uz: [
            "Dastaning yarmiga (1/2 qismiga) qadar",
            "Dastaning uchdan bir (1/3) qismiga qadar",
            "Dastaning to‘rtdan bir (1/4) qismiga qadar",
            "Dastaning eng quyi qismiga qadar"
        ],
        options_ru: [
            "До половины (1/2) длины древка",
            "На одну треть (1/3) длины древка",
            "На одну четверть (1/4) длины древка",
            "До самого основания древка"
        ],
        correct_index: 0,
    },
    InternalQuestion {
        id: 29,
        q_uz: "Davlat bayrog‘i boshqa davlatlar bayroqlari bilan bir vaqtda ko‘tarilganda uning o‘lchami qanday bo‘lishi shart?",
        q_ru: "Каким должен быть размер Государственного флага при одновременном поднятии с флагами других государств?",
        options_uz: [
            "Boshqa bayroqlardan kichik bo‘lmasligi va kamida ikki barobar pastda turmasligi shart",
            "Boshqa bayroqlardan kichikroq bo‘lishi mumkin",
            "Faqat oq fonda joylashtirilishi shart",
            "Boshqa bayroqlardan chap tomonda pastroq osilishi kerak"
        ],
        options_ru: [
            "Размер не может быть меньше других флагов, а высота — не ниже других",
            "Может быть меньше флагов других стран",
            "Должен размещаться только на белом фоне",
            "Должен располагаться ниже остальных флагов"
        ],
        correct_index: 0,
    },
    InternalQuestion {
        id: 30,
        q_uz: "O‘zbekiston Respublikasi Davlat bayrog‘i qabul qilingan kunga 2026-yil 18-noyabrda necha yil to‘ladi?",
        q_ru: "Сколько лет исполнится со дня принятия Государственного флага Республики Узбекистан 18 ноября 2026 года?",
        options_uz: ["33 yil", "34 yil", "36 yil", "35 yil"],
        options_ru: ["33 года", "34 года", "36 лет", "35 лет"],
        correct_index: 3,
    },
];

pub fn get_public_questions() -> Vec<PublicQuestion> {
    QUESTIONS
        .iter()
        .map(|q| PublicQuestion {
            id: q.id,
            q_uz: q.q_uz,
            q_ru: q.q_ru,
            options_uz: q.options_uz,
            options_ru: q.options_ru,
        })
        .collect()
}

pub async fn get_collection() -> Result<Collection<ParticipantResult>, mongodb::error::Error> {
    let client = MONGO_CLIENT
        .get_or_try_init(|| async {
            let uri = env::var("MONGODB_URI").expect("MONGODB_URI kiritilmagan!");
            Client::with_uri_str(&uri).await
        })
        .await?;

    let db = client.database("angren_flag_quiz");
    let coll = db.collection::<ParticipantResult>("results");

    // 60 ball bo'yicha kamayish va millisekund bo'yicha o'sish tartibida indeks
    let index = IndexModel::builder()
        .keys(doc! { "score": -1, "time_ms": 1 })
        .options(IndexOptions::builder().build())
        .build();
    let _ = coll.create_index(index).await;

    Ok(coll)
}

fn secret_key() -> String {
    env::var("QUIZ_SECRET_KEY").unwrap_or_else(|_| "angren_bayroq_2026_secret_key".to_string())
}

pub fn create_session_token(payload: &SessionPayload) -> String {
    let json = serde_json::to_string(payload).unwrap();
    let hex_data = hex::encode(json.as_bytes());
    let mut mac = HmacSha256::new_from_slice(secret_key().as_bytes()).unwrap();
    mac.update(hex_data.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    format!("{}.{}", hex_data, sig)
}

pub fn verify_session_token(token: &str) -> Option<SessionPayload> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 2 {
        return None;
    }
    let mut mac = HmacSha256::new_from_slice(secret_key().as_bytes()).ok()?;
    mac.update(parts[0].as_bytes());
    let expected_sig = hex::encode(mac.finalize().into_bytes());
    if expected_sig != parts[1] {
        return None;
    }
    let bytes = hex::decode(parts[0]).ok()?;
    serde_json::from_slice(&bytes).ok()
}
