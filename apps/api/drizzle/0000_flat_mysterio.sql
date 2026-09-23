CREATE TYPE "public"."command_state" AS ENUM('pending', 'completed', 'failed', 'timeout', 'conflict', 'rejected');--> statement-breakpoint
CREATE TABLE "audit_entries" (
	"id" bigserial PRIMARY KEY NOT NULL,
	"command_id" uuid NOT NULL,
	"principal" text NOT NULL,
	"kind" text NOT NULL,
	"state" "command_state" NOT NULL,
	"at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "commands" (
	"command_id" uuid PRIMARY KEY NOT NULL,
	"principal" text NOT NULL,
	"kind" text NOT NULL,
	"target" jsonb NOT NULL,
	"fingerprint" text NOT NULL,
	"state" "command_state" DEFAULT 'pending' NOT NULL,
	"verdict" jsonb,
	"classification" text,
	"detail" text,
	"response_status" integer,
	"response_body" jsonb,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	"terminal_at" timestamp with time zone
);
--> statement-breakpoint
CREATE UNIQUE INDEX "audit_entries_command_id_key" ON "audit_entries" USING btree ("command_id");--> statement-breakpoint
CREATE INDEX "audit_entries_at_idx" ON "audit_entries" USING btree ("at");--> statement-breakpoint
CREATE INDEX "commands_state_created_idx" ON "commands" USING btree ("state","created_at");