import type {
  MailAttachment,
  MailLabel,
  MailMessage,
  MailPerson,
  OrganizationalFolder,
} from "../types/mail";

export const SAMPLE_ADDRESS = "support@kamfany.com";
export const SAMPLE_NAME = "KAMFANY Support";

const desk: MailPerson = { name: SAMPLE_NAME, email: SAMPLE_ADDRESS };
const ada: MailPerson = { name: "Adaeze Okonkwo", email: "finance@kamfany.com" };
const malik: MailPerson = { name: "Malik Mensah", email: "developer@kamfany.com" };
const chinedu: MailPerson = { name: "Chinedu Okafor", email: "admin@kamfany.com" };
const ife: MailPerson = { name: "Ife Adeyemi", email: "info@kamfany.com" };
const amaka: MailPerson = { name: "Amaka Nwosu", email: "amaka@northwind.example" };
const jonah: MailPerson = { name: "Jonah Hale", email: "jonah@harbor.example" };
const priya: MailPerson = { name: "Priya Raman", email: "priya@lumen.example" };

export const sampleLabels: MailLabel[] = [
  { id: "finance", name: "Finance", tone: "brass" },
  { id: "clients", name: "Clients", tone: "paper" },
  { id: "internal", name: "Internal", tone: "mist" },
];

function hoursAgo(hours: number): string {
  return new Date(Date.now() - hours * 60 * 60 * 1000).toISOString();
}

function daysAhead(days: number): string {
  return new Date(Date.now() + days * 24 * 60 * 60 * 1000).toISOString();
}

function html(lines: string[]): string {
  return lines.map((line) => `<p>${line}</p>`).join("");
}

interface Draft {
  id: string;
  folder: OrganizationalFolder;
  from: MailPerson;
  to: MailPerson[];
  subject: string;
  preview: string;
  bodyText: string;
  receivedAt: string;
  cc?: MailPerson[];
  bcc?: MailPerson[];
  bodyHtml?: string | null;
  isRead?: boolean;
  isStarred?: boolean;
  isImportant?: boolean;
  snoozedUntil?: string | null;
  labelIds?: string[];
  attachments?: MailAttachment[];
}

function message(draft: Draft): MailMessage {
  return {
    cc: [],
    bcc: [],
    bodyHtml: null,
    isRead: true,
    isStarred: false,
    isImportant: false,
    snoozedUntil: null,
    labelIds: [],
    attachments: [],
    ...draft,
  };
}

const invoiceBody = [
  "Please find the March harbour invoice attached. The total is unchanged from the quote Adaeze approved last week.",
  "Payment is due on the 28th. Reply if the purchase order number needs to be added before then.",
];

const deployBody = [
  "The support desk deploy window is Thursday 06:00 to 06:40 WAT. Mail will keep arriving, and the reading pane may pause for a few minutes.",
  "I will send a note when the window closes. No action is needed unless you see a failed delivery after 07:00.",
];

const budgetBody = [
  "The desk budget for the next quarter is attached to the board pack, not this note. I wanted the support team to see the line for shared inboxes before Friday's review.",
  "Nothing here needs a reply unless the support address should stay on the finance copy list.",
];

const onboardingBody = [
  "Northwind is ready to move the client questions to support@kamfany.com from Monday. Ife already has the public hours.",
  "Could you confirm who watches the queue on Saturdays? We will put that name in the welcome note.",
];

const contractBody = [
  "Attached is the revised support schedule for Lumen. Section 4 now matches the hours we agreed on the call.",
  "If the wording is right, I will send the signature copy to info@kamfany.com tomorrow.",
];

const hoursBody = [
  "Public desk hours stay 08:00 to 17:00 WAT on weekdays. The Saturday rota is Malik in the morning and Ife after 12:00.",
  "Pin this for the new starters. It replaces the note from February.",
];

const signInText = [
  "A sign-in was reported for the support desk from a network we do not recognise.",
  "Do not send a password to anyone who asks for it. Reply on this thread if the attempt was not yours.",
  "The company security note is on the security page.",
].join("\n\n");

const signInHtml = [
  html([
    "A sign-in was reported for the support desk from a network we do not recognise.",
    "Do not send a password to anyone who asks for it. Reply on this thread if the attempt was not yours.",
    'The company security note is on <a href="https://kamfany.com/security">the security page</a>.',
  ]).replace("<p>", '<p onclick="steal()">'),
  "<script>window.stolen = document.cookie</script>",
  '<img src="https://tracking.example/pixel.gif" alt="">',
].join("");

const boardBody = [
  "The board asked for a one-page note on how the support desk handles billing questions. I starred it so it stays easy to find before Thursday.",
  "The attachment is the outline. Please add anything the night rota does differently.",
];

const followUpBody = [
  "Jonah asked for a call about the March invoice after the payment date. I snoozed this so it comes back on Monday morning.",
  "Nothing is overdue yet.",
];

const sentReplyBody = [
  "Saturday cover is Malik until 12:00 and Ife after that. We will watch the new queue from Monday.",
  "Send the welcome wording when you have it and we will reply from this address.",
];

const welcomeBody = [
  "The public address info@kamfany.com is live for general questions. Support still owns the client queue.",
  "Adaeze is copied privately so finance can see the handoff. Please do not forward this outside the company.",
];

const quoteDraft = [
  "Northwind asked for a quarterly support quote. I have the hours, and I still need the travel line from finance.",
  "Leaving this here until Adaeze replies.",
];

const replyDraft = [
  "Thanks for the revised schedule. I have read section 4 and will confirm once Malik checks the uptime line.",
];

const oldTicketBody = [
  "Closing the January access ticket. The mailbox grant was corrected and the client confirmed they can write in.",
  "Archived so the inbox stays on this month's work.",
];

const vendorBody = [
  "I compared the two inbox providers against the desk requirements. The short version is in the thread above this one.",
  "Starred so we can come back to it when the provider work starts. This is not a decision yet.",
];

const domainBody = [
  "Lumen's support domain renews on the 30th. The DNS contact is still priya@lumen.example.",
  "I can send the reminder to their admin if you want it to come from this desk.",
];

const meetingBody = [
  "Notes from this morning: keep the brass unread mark, do not add a second inbox for billing, and review the Saturday rota in two weeks.",
  "Chinedu will file the action list in the internal folder once that exists.",
];

export const sampleMessages: MailMessage[] = [
  message({
    id: "m01",
    folder: "inbox",
    from: jonah,
    to: [desk],
    subject: "March harbour invoice",
    preview: "The total is unchanged from the quote Adaeze approved.",
    bodyText: invoiceBody.join("\n\n"),
    bodyHtml: html(invoiceBody),
    receivedAt: hoursAgo(2),
    isRead: false,
    isImportant: true,
    labelIds: ["finance"],
    attachments: [
      {
        id: "a01",
        filename: "harbor-march-invoice.pdf",
        mimeType: "application/pdf",
        sizeBytes: 248_320,
      },
    ],
  }),
  message({
    id: "m02",
    folder: "inbox",
    from: malik,
    to: [desk],
    cc: [chinedu],
    subject: "Thursday deploy window",
    preview: "Mail keeps arriving. The reading pane may pause briefly.",
    bodyText: deployBody.join("\n\n"),
    bodyHtml: html(deployBody),
    receivedAt: hoursAgo(5),
    isRead: false,
    labelIds: ["internal"],
  }),
  message({
    id: "m03",
    folder: "inbox",
    from: ada,
    to: [desk],
    subject: "Desk budget before Friday",
    preview: "The support line is in the board pack, not this note.",
    bodyText: budgetBody.join("\n\n"),
    bodyHtml: html(budgetBody),
    receivedAt: hoursAgo(26),
    isStarred: true,
    labelIds: ["finance"],
  }),
  message({
    id: "m04",
    folder: "inbox",
    from: amaka,
    to: [desk],
    cc: [ife],
    subject: "Northwind queue from Monday",
    preview: "Who watches the queue on Saturdays?",
    bodyText: onboardingBody.join("\n\n"),
    bodyHtml: html(onboardingBody),
    receivedAt: hoursAgo(70),
    labelIds: ["clients"],
  }),
  message({
    id: "m05",
    folder: "inbox",
    from: priya,
    to: [desk],
    subject: "Revised Lumen support schedule",
    preview: "Section 4 now matches the hours from the call.",
    bodyText: contractBody.join("\n\n"),
    bodyHtml: html(contractBody),
    receivedAt: hoursAgo(6),
    isRead: false,
    labelIds: ["clients"],
    attachments: [
      {
        id: "a05",
        filename: "lumen-support-schedule.pdf",
        mimeType: "application/pdf",
        sizeBytes: 1_258_291,
      },
    ],
  }),
  message({
    id: "m06",
    folder: "inbox",
    from: chinedu,
    to: [desk, ife],
    subject: "Public desk hours",
    preview: "Weekdays 08:00 to 17:00. Saturday rota is posted.",
    bodyText: hoursBody.join("\n\n"),
    bodyHtml: html(hoursBody),
    receivedAt: hoursAgo(30),
    labelIds: ["internal"],
  }),
  message({
    id: "m07",
    folder: "inbox",
    from: chinedu,
    to: [desk],
    subject: "Suspicious sign-in on the support desk",
    preview: "Do not send a password to anyone who asks for it.",
    bodyText: signInText,
    bodyHtml: signInHtml,
    receivedAt: hoursAgo(1),
    isRead: false,
    isImportant: true,
    labelIds: ["internal"],
  }),
  message({
    id: "m08",
    folder: "inbox",
    from: ada,
    to: [desk],
    cc: [malik],
    subject: "Board note on billing questions",
    preview: "A one-page outline is attached for Thursday.",
    bodyText: boardBody.join("\n\n"),
    bodyHtml: html(boardBody),
    receivedAt: hoursAgo(50),
    isStarred: true,
    isImportant: true,
    labelIds: ["finance"],
    attachments: [
      {
        id: "a08",
        filename: "billing-questions-outline.pdf",
        mimeType: "application/pdf",
        sizeBytes: 184_320,
      },
    ],
  }),
  message({
    id: "m09",
    folder: "inbox",
    from: jonah,
    to: [desk],
    subject: "Follow up after the March payment",
    preview: "Snoozed until Monday. Nothing is overdue yet.",
    bodyText: followUpBody.join("\n\n"),
    bodyHtml: html(followUpBody),
    receivedAt: hoursAgo(20),
    snoozedUntil: daysAhead(2),
    labelIds: ["finance"],
  }),
  message({
    id: "m10",
    folder: "sent",
    from: desk,
    to: [amaka],
    cc: [ife],
    subject: "Re: Northwind queue from Monday",
    preview: "Saturday cover is Malik until 12:00 and Ife after that.",
    bodyText: sentReplyBody.join("\n\n"),
    bodyHtml: html(sentReplyBody),
    receivedAt: hoursAgo(4),
  }),
  message({
    id: "m11",
    folder: "sent",
    from: desk,
    to: [ife],
    bcc: [ada],
    subject: "Public address is live",
    preview: "info@kamfany.com is live. Support still owns the client queue.",
    bodyText: welcomeBody.join("\n\n"),
    bodyHtml: html(welcomeBody),
    receivedAt: hoursAgo(80),
  }),
  message({
    id: "m12",
    folder: "drafts",
    from: desk,
    to: [amaka],
    subject: "Quarterly support quote",
    preview: "Hours are in. The travel line is still missing.",
    bodyText: quoteDraft.join("\n\n"),
    receivedAt: hoursAgo(9),
    isRead: false,
    labelIds: ["clients"],
  }),
  message({
    id: "m13",
    folder: "drafts",
    from: desk,
    to: [priya],
    subject: "Re: Revised Lumen support schedule",
    preview: "Waiting on Malik for the uptime line.",
    bodyText: replyDraft.join("\n\n"),
    receivedAt: hoursAgo(3),
    isRead: false,
  }),
  message({
    id: "m14",
    folder: "archive",
    from: amaka,
    to: [desk],
    subject: "January access ticket",
    preview: "The mailbox grant was corrected and the client confirmed.",
    bodyText: oldTicketBody.join("\n\n"),
    bodyHtml: html(oldTicketBody),
    receivedAt: hoursAgo(200),
    labelIds: ["clients"],
  }),
  message({
    id: "m15",
    folder: "archive",
    from: malik,
    to: [desk],
    subject: "Inbox provider comparison",
    preview: "A short comparison, not a decision.",
    bodyText: vendorBody.join("\n\n"),
    bodyHtml: html(vendorBody),
    receivedAt: hoursAgo(300),
    isStarred: true,
    isImportant: true,
    labelIds: ["internal"],
  }),
  message({
    id: "m16",
    folder: "spam",
    from: { name: "Prize Desk", email: "prize@reward.example" },
    to: [desk],
    subject: "You have been selected",
    preview: "Unsolicited prize notice. Left in spam.",
    bodyText:
      "This message claims a prize and asks for bank details. It was left in spam and should stay there.",
    receivedAt: hoursAgo(15),
    isRead: false,
  }),
  message({
    id: "m17",
    folder: "spam",
    from: { name: "Harbor Billing", email: "billing@harbor-secure.example" },
    to: [desk],
    subject: "Invoice overdue — pay today",
    preview: "The sending domain is not Harbor. Left in spam.",
    bodyText:
      "The attachment name looks like a Harbour invoice, but the address is not jonah@harbor.example. It stays in spam.",
    receivedAt: hoursAgo(11),
    isRead: false,
    attachments: [
      {
        id: "a17",
        filename: "invoice-overdue.pdf",
        mimeType: "application/pdf",
        sizeBytes: 88_064,
      },
    ],
  }),
  message({
    id: "m18",
    folder: "trash",
    from: { name: "Lumen News", email: "news@lumen.example" },
    to: [desk],
    subject: "Product news for March",
    preview: "A newsletter moved to trash.",
    bodyText:
      "Monthly product news from Lumen. It was moved to trash from the support desk.",
    receivedAt: hoursAgo(90),
  }),
  message({
    id: "m19",
    folder: "inbox",
    from: priya,
    to: [desk],
    subject: "Lumen domain renewal",
    preview: "The support domain renews on the 30th.",
    bodyText: domainBody.join("\n\n"),
    bodyHtml: html(domainBody),
    receivedAt: hoursAgo(8),
    isRead: false,
    labelIds: ["clients"],
  }),
  message({
    id: "m20",
    folder: "inbox",
    from: chinedu,
    to: [desk],
    subject: "Notes from this morning",
    preview: "Keep one inbox for billing. Review the Saturday rota later.",
    bodyText: meetingBody.join("\n\n"),
    bodyHtml: html(meetingBody),
    receivedAt: hoursAgo(12),
    labelIds: ["internal"],
  }),
];
