import { db } from '.';

export interface Journal {
  id: string;
  date: string;
  content?: string;
  color?: string;
  createdAt: Date;
  updatedAt: Date;
}

function dateToString(date: Date): string {
  return date.toISOString();
}

function stringToDate(dateString: string): Date {
  return new Date(dateString);
}

export async function writeJournal(data: Journal): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  await db.execute(
    `INSERT OR REPLACE INTO journal (id, date, content, color, createdAt, updatedAt)
     VALUES ($1, $2, $3, $4, $5, $6)`,
    [
      data.id,
      data.date,
      data.content ?? null,
      data.color ?? null,
      dateToString(data.createdAt),
      dateToString(data.updatedAt),
    ]
  );
}

export async function getJournalByDate(date: string): Promise<Journal | undefined> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<Journal[]>(
    `SELECT * FROM journal WHERE date = $1`,
    [date]
  );
  if (result.length === 0) return undefined;
  const row = result[0] as any;
  return {
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
  };
}

export async function updateJournal(id: string, data: Partial<Journal>): Promise<void> {
  if (!db) throw new Error('Database not initialized');

  const existing = await getJournalByDate(id);
  if (!existing) throw new Error(`Journal with date ${id} not found`);

  const updated = { ...existing, ...data, updatedAt: new Date() };
  await writeJournal(updated);
}

export async function deleteJournal(date: string): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  await db.execute('DELETE FROM journal WHERE date = $1', [date]);
}

export interface JournalDateInfo {
  date: string;
  color?: string;
}

export async function getJournalDatesInMonth(year: number, month: number): Promise<JournalDateInfo[]> {
  if (!db) throw new Error('Database not initialized');
  
  const monthStr = String(month).padStart(2, '0');
  const startDate = `${year}-${monthStr}-01`;
  const endDate = `${year}-${monthStr}-31`;
  
  const result = await db.select<JournalDateInfo[]>(
    `SELECT date, color FROM journal 
     WHERE date >= $1 AND date <= $2
     AND content IS NOT NULL AND content != ''
     ORDER BY date ASC`,
    [startDate, endDate]
  );
  
  return result;
}

export async function getRecentJournals(sinceDate: string): Promise<Journal[]> {
  if (!db) throw new Error('Database not initialized');
  
  const result = await db.select<Journal[]>(
    `SELECT * FROM journal 
     WHERE date >= $1 
       AND content IS NOT NULL AND content != ''
     ORDER BY updatedAt DESC`,
    [sinceDate]
  );
  
  return result.map((row: any) => ({
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
  }));
}
