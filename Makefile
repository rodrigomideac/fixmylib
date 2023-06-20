setup_db:
	docker start postgres-14 || docker run --rm -d --name postgres-14 -p 5432:5432 -e POSTGRES_PASSWORD=fixmylib postgres:14
	PGPASSWORD=fixmylib psql -h localhost -U postgres -d postgres -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public"
	sqlx migrate run
