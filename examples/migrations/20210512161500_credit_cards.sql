CREATE TABLE credit_cards(
    "payment_id" UUID PRIMARY KEY NOT NULL,
    "credit_card_id" UUID NOT NULl,
    "credit_card_payment_type" TEXT NOT NULL,
    "amount" INTEGER NOT NULL
);
