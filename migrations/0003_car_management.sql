-- Vehicles table
CREATE TABLE IF NOT EXISTS vehicles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL, 
    model TEXT,        
    plate TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Fuel logs
CREATE TABLE IF NOT EXISTS fuel_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vehicle_id INTEGER NOT NULL,
    date DATETIME DEFAULT CURRENT_TIMESTAMP,
    odometer REAL NOT NULL, 
    liters REAL NOT NULL,
    price_per_liter REAL NOT NULL,
    fuel_type TEXT CHECK(fuel_type IN ('gasoline', 'alcohol')) NOT NULL,
    total_cost REAL NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id) ON DELETE CASCADE
);

-- General expenses
CREATE TABLE IF NOT EXISTS expense_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vehicle_id INTEGER NOT NULL,
    date DATETIME DEFAULT CURRENT_TIMESTAMP,
    category TEXT CHECK(category IN ('maintenance', 'tax', 'parking', 'toll', 'insurance', 'other')) NOT NULL,
    description TEXT,
    cost REAL NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id) ON DELETE CASCADE
);
