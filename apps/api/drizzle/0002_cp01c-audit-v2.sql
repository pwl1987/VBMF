ALTER TABLE "audit_entries" ALTER COLUMN "action" SET NOT NULL;--> statement-breakpoint
ALTER TABLE "audit_entries" ALTER COLUMN "decision" SET NOT NULL;--> statement-breakpoint
ALTER TABLE "audit_entries" ALTER COLUMN "command_id" DROP NOT NULL;--> statement-breakpoint
ALTER TABLE "audit_entries" ALTER COLUMN "state" DROP NOT NULL;--> statement-breakpoint
ALTER TABLE "audit_entries" DROP COLUMN "kind";