import { UserService, User, createUser } from './user_service';

class App {
    constructor() {
        this.userService = new UserService();
        this.init();
    }
    
    init() {
        const admin = createUser('admin', 'Administrator', 'admin@example.com');
        this.userService.addUser(admin);
        
        const guest = createUser('guest', 'Guest User', 'guest@example.com');
        this.userService.addUser(guest);
        
        console.log('App initialized with users');
    }
    
    run() {
        const users = this.userService.getAllUsers();
        users.forEach(user => {
            console.log(`User: ${user.name} (${user.email})`);
        });
    }
}

// Global variable
const APP_CONFIG = {
    debug: true,
    version: '1.0.0'
};

// Export the App class
export default App;