// Help text for every screen and the welcome tour, in English, Hindi and Marathi.
"use strict";

const HELP = {
  en: {
    _title: "How to use this screen",
    projects: [
      "New business: type its name, choose the type and the year, press <b>Create</b>.",
      "Already created: press <b>Open</b> in the list below.",
      "Next year of the same business: create it with the same name and the new year. Your choices are carried forward.",
      "<b>Delete</b> moves a year to the Recycle Bin. You can restore it from there.",
    ],
    import: [
      "Tally users: open the company in Tally, press <b>Find companies</b>, choose it, press <b>Import</b>.",
      "Others: press <b>Choose file</b> against the trial balance (Excel or CSV). The other files are optional but give better checks.",
      "Wrong file? Press <b>Remove</b> or <b>Replace</b>. A file that cannot be read is never accepted.",
      "Then press <b>Next: run the checks</b> (top right).",
    ],
    check: [
      "Press <b>Run checks</b>. It takes a few seconds.",
      "<b>Must fix</b> (red): the final copy is locked until these are solved. <b>Check</b>: look at it. <b>Note</b>: for information.",
      "Read <b>What to do</b> under each item. Fix it in your books and import again, or correct it here (Map ledgers or Adjustments).",
      "<b>Explain</b> gives a simple explanation from the local AI (if installed).",
    ],
    map: [
      "Each ledger is placed under a line of the Balance Sheet or Profit and Loss. <b>Certain</b>: only one line is possible. <b>Confirm</b>: LedgerCraft's proposal, which you must check. <b>Review</b>: your earlier choice, but the books changed.",
      "To change, pick a new line in <b>Shown under</b>. It is saved at once and remembered for next year.",
      "<b>Reset</b> goes back to LedgerCraft's own choice. Tags (MSME, transporter …) change how checks and notes treat the ledger.",
    ],
    adjust: [
      "Use this for entries not in the books: audit fee payable, depreciation, rectifications, closing stock.",
      "Press <b>New adjustment</b>, write why, enter debit and credit lines. Debit total must equal credit total.",
      "A ledger that is not in the books is created: choose its group.",
      "<b>Switch off</b> keeps the entry but stops it; <b>Delete</b> removes it. Every change is in the audit trail. Pass the same entries in your books later.",
    ],
    present: [
      "Switch on or off what to print: for example turn off the Cash Flow Statement.",
      "Choose the unit (rupees, thousands, lakhs …). Totals always add up exactly after rounding.",
      "Press <b>Save and preview</b> (top right). The preview shows the pages as they will print.",
      "<b>Reset to standard</b> brings back the usual choices.",
    ],
    export: [
      "Fill auditor details and the people signing for the business. Paste the UDIN when you have it.",
      "Choose <b>Draft</b> to share for discussion, or <b>Final</b> when every <b>Must fix</b> item is cleared.",
      "Tick the files you want and press <b>Save and export</b>. Then press <b>Open export folder</b>.",
    ],
    analysis: [
      "Key figures compare this year with last year.",
      "<b>Items for tax audit</b> counts cash payments, cash receipts and loans in cash. Press <b>See list</b> for each case.",
      "<b>Save auditor workbook</b> (top right) writes everything to an Excel file for the auditor.",
    ],
    disclose: [
      "Open each tab and fill what applies. Companies must fill <b>Share capital</b>; the total must agree with the books.",
      "<b>Accounting policies</b> shows the standard wording; change any of it for this entity.",
      "Press <b>Save disclosures</b> (top right), then <b>Preview</b> to see them in the notes.",
    ],
    audit: [
      "This list shows who did what and when, newest first.",
      "The green line means no entry was edited or removed in between. A red line means the record was changed. Keep the exported files as your outside copy.",
    ],
    tour: [
      ["Welcome to LedgerCraft", "First choose what you want to do: make financial statements, check and analyse books, or get tax audit help. It turns your books into financial statements in the format required by law, checks the books like a CA, and prepares a workbook for the auditor. Everything stays on this computer."],
      ["1. Create the client and year", "On the first screen type the business name, its type (firm, company, LLP …) and the year. Press Create."],
      ["2. Bring in the books", "From Tally in one click, or upload the trial balance from Excel, Zoho Books or BUSY. Last year's balances and the day book are optional but help the checks."],
      ["3. Check, map and adjust", "Run checks and read 'What to do' under each item. Correct the placement of any ledger once (it is remembered). Pass any adjustment entries."],
      ["4. Print and sign", "Choose what to print and the units, look at the preview, fill the signing details and export. PDF and Excel land in one folder."],
      ["Always on your side", "Steps are on the left; the main buttons are always at the top right of each screen. 'What to do now' at the bottom left tells you the next step. Every change is recorded in the audit trail."],
    ],
    next: { intent: "Choose what you want to do.", analyse: "Look at the analysis and save the auditor workbook.", create: "Create or open a client.", tb: "Import the trial balance.", run: "Run the checks.", fix: n => `Solve ${n} "Must fix" item(s).`, map: n => `${n} ledger placement(s) need your confirmation.`, ready: "Look at the preview, then sign and export." },
  },
  hi: {
    _title: "इस स्क्रीन का उपयोग कैसे करें",
    projects: [
      "नया व्यवसाय: नाम लिखें, प्रकार और वर्ष चुनें, <b>Create</b> दबाएँ।",
      "पहले से बना है: नीचे की सूची में <b>Open</b> दबाएँ।",
      "उसी व्यवसाय का अगला वर्ष: वही नाम और नया वर्ष देकर बनाएँ। आपकी पसंद आगे ले जाई जाती है।",
      "<b>Delete</b> वर्ष को Recycle Bin में भेजता है। वहाँ से वापस ला सकते हैं।",
    ],
    import: [
      "Tally: Tally में कंपनी खोलें, <b>Find companies</b> दबाएँ, कंपनी चुनें, <b>Import</b> दबाएँ।",
      "अन्य: ट्रायल बैलेंस के सामने <b>Choose file</b> दबाएँ (Excel या CSV)। बाकी फ़ाइलें वैकल्पिक हैं पर जाँच बेहतर करती हैं।",
      "गलत फ़ाइल? <b>Remove</b> या <b>Replace</b> दबाएँ। जो फ़ाइल पढ़ी न जा सके वह कभी स्वीकार नहीं होती।",
      "फिर ऊपर दाईं ओर <b>Next: run the checks</b> दबाएँ।",
    ],
    check: [
      "<b>Run checks</b> दबाएँ। कुछ सेकंड लगते हैं।",
      "<b>Must fix</b> (लाल): इन्हें सुधारे बिना अंतिम प्रति नहीं बनेगी। <b>Check</b>: देख लें। <b>Note</b>: जानकारी के लिए।",
      "हर बिंदु के नीचे <b>What to do</b> पढ़ें। किताबों में सुधार कर फिर import करें, या यहीं सुधारें (Map ledgers या Adjustments)।",
      "<b>Explain</b> स्थानीय AI (यदि लगा हो) से सरल भाषा में समझाता है।",
    ],
    map: [
      "हर लेजर बैलेंस शीट या लाभ-हानि की किसी पंक्ति में रखा जाता है। <b>Certain</b>: केवल एक पंक्ति संभव। <b>Confirm</b>: LedgerCraft का प्रस्ताव, जिसे आपको जाँचना है। <b>Review</b>: आपकी पिछली पसंद, पर किताबें बदल गईं।",
      "बदलने के लिए <b>Shown under</b> में नई पंक्ति चुनें। तुरंत सेव होता है और अगले वर्ष भी याद रहता है।",
      "<b>Reset</b> LedgerCraft की अपनी पसंद पर लौटाता है। टैग (MSME, transporter …) जाँच और नोट्स को बदलते हैं।",
    ],
    adjust: [
      "जो प्रविष्टियाँ किताबों में नहीं हैं उनके लिए: ऑडिट फीस देय, मूल्यह्रास, सुधार प्रविष्टि, अंतिम स्टॉक।",
      "<b>New adjustment</b> दबाएँ, कारण लिखें, नामे और जमा पंक्तियाँ भरें। नामे का जोड़ जमा के जोड़ के बराबर होना चाहिए।",
      "जो लेजर किताबों में नहीं है वह बनाया जाता है: उसका ग्रुप चुनें।",
      "<b>Switch off</b> प्रविष्टि रखता है पर लागू नहीं करता; <b>Delete</b> हटा देता है। हर बदलाव ऑडिट ट्रेल में दर्ज है। बाद में यही प्रविष्टियाँ किताबों में भी करें।",
    ],
    present: [
      "क्या छापना है उसे चालू या बंद करें: जैसे Cash Flow Statement बंद करें।",
      "इकाई चुनें (रुपये, हज़ार, लाख …)। राउंडिंग के बाद भी जोड़ हमेशा सही मिलता है।",
      "ऊपर दाईं ओर <b>Save and preview</b> दबाएँ। पूर्वावलोकन में पन्ने वैसे ही दिखते हैं जैसे छपेंगे।",
      "<b>Reset to standard</b> सामान्य विकल्प वापस लाता है।",
    ],
    export: [
      "ऑडिटर का विवरण और व्यवसाय की ओर से हस्ताक्षर करने वालों के नाम भरें। UDIN मिलने पर चिपकाएँ।",
      "चर्चा के लिए <b>Draft</b> चुनें, या सभी <b>Must fix</b> सुधरने पर <b>Final</b>।",
      "चाहिए वे फ़ाइलें चुनें और <b>Save and export</b> दबाएँ। फिर <b>Open export folder</b> दबाएँ।",
    ],
    analysis: [
      "मुख्य आँकड़े इस वर्ष की तुलना पिछले वर्ष से करते हैं।",
      "<b>Items for tax audit</b> नकद भुगतान, नकद प्राप्ति और नकद ऋण गिनता है। हर मामले के लिए <b>See list</b> दबाएँ।",
      "ऊपर दाईं ओर <b>Save auditor workbook</b> सब कुछ ऑडिटर के लिए Excel फ़ाइल में लिखता है।",
    ],
    disclose: [
      "हर टैब खोलें और जो लागू हो वह भरें। कंपनियों को <b>Share capital</b> भरना ज़रूरी है; जोड़ किताबों से मिलना चाहिए।",
      "<b>Accounting policies</b> में मानक शब्द दिखते हैं; इस इकाई के लिए कोई भी बदल सकते हैं।",
      "ऊपर दाईं ओर <b>Save disclosures</b> दबाएँ, फिर नोट्स में देखने के लिए <b>Preview</b>।",
    ],
    audit: [
      "यह सूची बताती है किसने क्या और कब किया, नया सबसे ऊपर।",
      "हरी पंक्ति का अर्थ है बाद में कुछ बदला या हटाया नहीं गया। लाल पंक्ति का अर्थ है रिकॉर्ड से छेड़छाड़ हुई।",
    ],
    tour: [
      ["LedgerCraft में स्वागत है", "पहले चुनें कि क्या करना है: वित्तीय विवरण बनाना, किताबों की जाँच और विश्लेषण, या टैक्स ऑडिट सहायता। यह आपकी किताबों से कानून के अनुसार प्रारूप में वित्तीय विवरण बनाता है, CA की तरह जाँच करता है, और ऑडिटर के लिए वर्कबुक तैयार करता है। सब कुछ इसी कंप्यूटर पर रहता है।"],
      ["1. क्लाइंट और वर्ष बनाएँ", "पहली स्क्रीन पर व्यवसाय का नाम, प्रकार (फर्म, कंपनी, LLP …) और वर्ष लिखें। Create दबाएँ।"],
      ["2. किताबें लाएँ", "Tally से एक क्लिक में, या Excel, Zoho Books या BUSY का ट्रायल बैलेंस अपलोड करें। पिछले वर्ष के बैलेंस और डे बुक वैकल्पिक हैं पर जाँच में मदद करते हैं।"],
      ["3. जाँच, मैपिंग और समायोजन", "जाँच चलाएँ और हर बिंदु के नीचे 'What to do' पढ़ें। किसी लेजर की जगह एक बार सुधारें (याद रहती है)। ज़रूरी समायोजन प्रविष्टियाँ करें।"],
      ["4. छापें और हस्ताक्षर", "क्या छापना है और इकाई चुनें, पूर्वावलोकन देखें, हस्ताक्षर विवरण भरें और export करें। PDF और Excel एक ही फ़ोल्डर में आते हैं।"],
      ["हमेशा आपके साथ", "चरण बाईं ओर हैं; मुख्य बटन हर स्क्रीन पर ऊपर दाईं ओर हैं। नीचे बाईं ओर 'What to do now' अगला चरण बताता है। हर बदलाव ऑडिट ट्रेल में दर्ज होता है।"],
    ],
    next: { intent: "आप क्या करना चाहते हैं, चुनें।", analyse: "विश्लेषण देखें और ऑडिटर वर्कबुक सेव करें।", create: "क्लाइंट बनाएँ या खोलें।", tb: "ट्रायल बैलेंस import करें।", run: "जाँच चलाएँ।", fix: n => `${n} "Must fix" बिंदु सुधारें।`, map: n => `${n} लेजर की जगह की पुष्टि करनी है।`, ready: "पूर्वावलोकन देखें, फिर हस्ताक्षर कर export करें।" },
  },
  mr: {
    _title: "ही स्क्रीन कशी वापरायची",
    projects: [
      "नवीन व्यवसाय: नाव लिहा, प्रकार आणि वर्ष निवडा, <b>Create</b> दाबा.",
      "आधीच तयार आहे: खालच्या यादीत <b>Open</b> दाबा.",
      "त्याच व्यवसायाचे पुढचे वर्ष: तेच नाव आणि नवीन वर्ष देऊन तयार करा. तुमच्या निवडी पुढे नेल्या जातात.",
      "<b>Delete</b> वर्ष Recycle Bin मध्ये पाठवते. तिथून परत आणता येते.",
    ],
    import: [
      "Tally: Tally मध्ये कंपनी उघडा, <b>Find companies</b> दाबा, कंपनी निवडा, <b>Import</b> दाबा.",
      "इतर: ट्रायल बॅलन्ससमोर <b>Choose file</b> दाबा (Excel किंवा CSV). बाकी फाइल्स ऐच्छिक आहेत पण तपासणी चांगली होते.",
      "चुकीची फाइल? <b>Remove</b> किंवा <b>Replace</b> दाबा. न वाचता येणारी फाइल कधीच स्वीकारली जात नाही.",
      "मग वर उजवीकडे <b>Next: run the checks</b> दाबा.",
    ],
    check: [
      "<b>Run checks</b> दाबा. काही सेकंद लागतात.",
      "<b>Must fix</b> (लाल): हे सुधारल्याशिवाय अंतिम प्रत बनणार नाही. <b>Check</b>: पाहून घ्या. <b>Note</b>: माहितीसाठी.",
      "प्रत्येक मुद्द्याखाली <b>What to do</b> वाचा. पुस्तकांत दुरुस्ती करून पुन्हा import करा, किंवा इथेच दुरुस्त करा (Map ledgers किंवा Adjustments).",
      "<b>Explain</b> स्थानिक AI (असल्यास) कडून सोप्या भाषेत समजावते.",
    ],
    map: [
      "प्रत्येक लेजर ताळेबंद किंवा नफा-तोटा पत्रकातील एका ओळीखाली ठेवला जातो. <b>Certain</b>: एकच ओळ शक्य. <b>Confirm</b>: LedgerCraft चा प्रस्ताव, जो तुम्ही तपासायचा. <b>Review</b>: तुमची आधीची निवड, पण पुस्तके बदलली.",
      "बदलायचे असल्यास <b>Shown under</b> मध्ये नवीन ओळ निवडा. लगेच जतन होते आणि पुढच्या वर्षीही लक्षात राहते.",
      "<b>Reset</b> LedgerCraft च्या स्वतःच्या निवडीकडे परत नेते. टॅग (MSME, transporter …) तपासणी आणि नोट्स बदलतात.",
    ],
    adjust: [
      "पुस्तकांत नसलेल्या नोंदींसाठी: देय ऑडिट फी, घसारा, दुरुस्ती नोंदी, अखेरचा साठा.",
      "<b>New adjustment</b> दाबा, कारण लिहा, नावे आणि जमा ओळी भरा. नावे बेरीज जमा बेरजेइतकी हवी.",
      "पुस्तकांत नसलेला लेजर तयार केला जातो: त्याचा ग्रुप निवडा.",
      "<b>Switch off</b> नोंद ठेवते पण लागू करत नाही; <b>Delete</b> काढून टाकते. प्रत्येक बदल ऑडिट ट्रेलमध्ये नोंदवला जातो. नंतर याच नोंदी पुस्तकांतही करा.",
    ],
    present: [
      "काय छापायचे ते चालू किंवा बंद करा: उदा. Cash Flow Statement बंद करा.",
      "एकक निवडा (रुपये, हजार, लाख …). राउंडिंगनंतरही बेरीज नेहमी अचूक जुळते.",
      "वर उजवीकडे <b>Save and preview</b> दाबा. पूर्वावलोकनात पाने छापल्यासारखी दिसतात.",
      "<b>Reset to standard</b> नेहमीच्या निवडी परत आणते.",
    ],
    export: [
      "ऑडिटरचा तपशील आणि व्यवसायातर्फे सही करणाऱ्यांची नावे भरा. UDIN मिळाल्यावर पेस्ट करा.",
      "चर्चेसाठी <b>Draft</b> निवडा, किंवा सर्व <b>Must fix</b> सुटल्यावर <b>Final</b>.",
      "हव्या त्या फाइल्स निवडा आणि <b>Save and export</b> दाबा. मग <b>Open export folder</b> दाबा.",
    ],
    analysis: [
      "मुख्य आकडे या वर्षाची तुलना मागील वर्षाशी करतात.",
      "<b>Items for tax audit</b> रोख देयके, रोख प्राप्ती आणि रोख कर्जे मोजते. प्रत्येक प्रकरणासाठी <b>See list</b> दाबा.",
      "वर उजवीकडे <b>Save auditor workbook</b> सर्व काही ऑडिटरसाठी Excel फाइलमध्ये लिहिते.",
    ],
    disclose: [
      "प्रत्येक टॅब उघडा आणि लागू असेल ते भरा. कंपन्यांनी <b>Share capital</b> भरणे आवश्यक आहे; बेरीज पुस्तकांशी जुळली पाहिजे.",
      "<b>Accounting policies</b> मध्ये मानक मजकूर दिसतो; या संस्थेसाठी कोणताही बदलू शकता.",
      "वर उजवीकडे <b>Save disclosures</b> दाबा, मग नोट्समध्ये पाहण्यासाठी <b>Preview</b>.",
    ],
    audit: [
      "ही यादी कोणी काय आणि केव्हा केले ते दाखवते, नवीन सर्वात वर.",
      "हिरवी ओळ म्हणजे नंतर काहीही बदलले किंवा काढले नाही. लाल ओळ म्हणजे नोंदीत फेरफार झाला.",
    ],
    tour: [
      ["LedgerCraft मध्ये स्वागत", "आधी काय करायचे ते निवडा: आर्थिक विवरणपत्रे बनवणे, पुस्तकांची तपासणी आणि विश्लेषण, किंवा टॅक्स ऑडिट मदत. हे तुमच्या पुस्तकांवरून कायद्याप्रमाणे नमुन्यात आर्थिक विवरणपत्रे बनवते, CA प्रमाणे तपासणी करते आणि ऑडिटरसाठी वर्कबुक तयार करते. सर्व काही याच संगणकावर राहते."],
      ["1. क्लायंट आणि वर्ष तयार करा", "पहिल्या स्क्रीनवर व्यवसायाचे नाव, प्रकार (फर्म, कंपनी, LLP …) आणि वर्ष लिहा. Create दाबा."],
      ["2. पुस्तके आणा", "Tally मधून एका क्लिकमध्ये, किंवा Excel, Zoho Books किंवा BUSY चा ट्रायल बॅलन्स अपलोड करा. मागील वर्षाचे बॅलन्स आणि डे बुक ऐच्छिक आहेत पण तपासणीला मदत करतात."],
      ["3. तपासणी, मॅपिंग आणि समायोजन", "तपासणी चालवा आणि प्रत्येक मुद्द्याखाली 'What to do' वाचा. एखाद्या लेजरची जागा एकदा दुरुस्त करा (लक्षात राहते). आवश्यक समायोजन नोंदी करा."],
      ["4. छापा आणि सही करा", "काय छापायचे आणि एकक निवडा, पूर्वावलोकन पाहा, सहीचा तपशील भरा आणि export करा. PDF आणि Excel एकाच फोल्डरमध्ये येतात."],
      ["नेहमी तुमच्यासोबत", "पायऱ्या डावीकडे आहेत; मुख्य बटणे प्रत्येक स्क्रीनवर वर उजवीकडे आहेत. खाली डावीकडे 'What to do now' पुढची पायरी सांगते. प्रत्येक बदल ऑडिट ट्रेलमध्ये नोंदवला जातो."],
    ],
    next: { intent: "तुम्हाला काय करायचे आहे ते निवडा.", analyse: "विश्लेषण पाहा आणि ऑडिटर वर्कबुक जतन करा.", create: "क्लायंट तयार करा किंवा उघडा.", tb: "ट्रायल बॅलन्स import करा.", run: "तपासणी चालवा.", fix: n => `${n} "Must fix" मुद्दे सोडवा.`, map: n => `${n} लेजरच्या जागेची पुष्टी करायची आहे.`, ready: "पूर्वावलोकन पाहा, मग सही करून export करा." },
  },
};
