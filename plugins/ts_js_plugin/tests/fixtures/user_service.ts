export class UserService {
    private users: Map<string, User> = new Map();
    
    constructor() {
        console.log("UserService initialized");
    }
    
    public addUser(user: User): void {
        this.users.set(user.id, user);
    }
    
    public getUser(id: string): User | undefined {
        return this.users.get(id);
    }
    
    public getAllUsers(): User[] {
        return Array.from(this.users.values());
    }
}

export interface User {
    id: string;
    name: string;
    email: string;
}

export function createUser(id: string, name: string, email: string): User {
    return { id, name, email };
}

const DEFAULT_USER: User = {
    id: "default",
    name: "Default User",
    email: "default@example.com"
};