-- Add user_id to vehicles and documents for data isolation
ALTER TABLE vehicles ADD COLUMN user_id INTEGER;
ALTER TABLE documents ADD COLUMN user_id INTEGER;

-- Assign existing records to the current owner
UPDATE vehicles SET user_id = 171600982 WHERE user_id IS NULL;
UPDATE documents SET user_id = 171600982 WHERE user_id IS NULL;
