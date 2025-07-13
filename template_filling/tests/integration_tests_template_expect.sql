-- template_demo.[part].part_1_export.sql.[v].1.tmpl

1
-- The defaut value of indent_base is tag

-- The kind of Symbol has: {%..%} {$..$} {{..}} {#..#}, and it only a Raw Token demo

/*
Create by Gin,Gin2,Gin3
*/

-- Table x_demo_table
CREATE TABLE x_demo_table (
-- This is indent_base=raw demo 1: no indent
    -- This is indent_base=raw demo 2: 4 space indent
    id SERIAL PRIMARY KEY,
    first_name VARCHAR(50) NOT NULL,
    last_name VARCHAR(50) NOT NULL,
    -- Default indent_base is 'inherit', here is 'tag' because of global env define is 'tag'
    -- After 'env' Token
    -- $index is 0, $max is 1
    hobby varchar(100), -- personal hobby
    -- After 'env' Token
    -- $index is 1, $max is 1
    address varchar(1000), -- personal address
    -- [TestCase] first item in tag, should be fill the indent
            -- ending text demo: last index is 1, max is 1
              -- This is test case for nested indentation-- Text after Tag 'for', use to test token context attribute 'end_of_row'
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Table x_demo_table comment
COMMENT ON TABLE x_demo_table IS 'This is a template filling demo table';
COMMENT ON COLUMN x_demo_table.id IS 'Identifier';
COMMENT ON COLUMN x_demo_table.first_name IS 'First name';
COMMENT ON COLUMN x_demo_table.last_name IS 'Last name';
-- Define custom indent_base as 'raw', indent based on the literal indentation
  COMMENT ON COLUMN x_demo_table.hobby IS 'personal hobby';
  COMMENT ON COLUMN x_demo_table.address IS 'personal address';
COMMENT ON COLUMN x_demo_table.created_at IS 'Created at';
COMMENT ON COLUMN x_demo_table.updated_at IS 'Updated at';

-- Case Tag::If
-- Pass. "abc" == "abc" is true

-- Single line join
aaa,bbb,ccc
-- Mixed char and line feed join
aaa,
bbb,
ccc

[Pass] Tag::If single boolean condition
[Pass] Tag::If single boolean variable condition
[Pass] Tag::If "A" == "B" condition
-- end text
