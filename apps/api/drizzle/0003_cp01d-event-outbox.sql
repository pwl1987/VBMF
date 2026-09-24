CREATE TABLE "event_outbox" (
	"sequence" bigserial PRIMARY KEY NOT NULL,
	"observed_at" timestamp with time zone DEFAULT now() NOT NULL,
	"snapshot" jsonb NOT NULL
);
--> statement-breakpoint
CREATE INDEX "event_outbox_observed_at_idx" ON "event_outbox" USING btree ("observed_at");