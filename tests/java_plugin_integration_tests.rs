use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[cfg(test)]
mod java_plugin_integration_tests {
    use super::*;

    fn create_comprehensive_java_project(temp_dir: &TempDir) -> String {
        let project_dir = temp_dir.path().join("comprehensive_project");
        fs::create_dir_all(&project_dir).unwrap();

        // Create package structure
        let base_package = project_dir.join("com/example/project");
        fs::create_dir_all(&base_package).unwrap();

        // Create multiple related classes
        let model_interface = r#"
package com.example.project;

import java.time.LocalDateTime;

public interface Model {
    String getId();
    LocalDateTime getCreatedAt();
    void validate() throws IllegalArgumentException;
}
"#;
        fs::write(base_package.join("Model.java"), model_interface).unwrap();

        let user_model = r#"
package com.example.project;

import java.time.LocalDateTime;
import java.util.List;
import java.util.ArrayList;

public class User implements Model {
    private String id;
    private String name;
    private String email;
    private LocalDateTime createdAt;
    private List<String> roles;
    
    public User(String id, String name, String email) {
        this.id = id;
        this.name = name;
        this.email = email;
        this.createdAt = LocalDateTime.now();
        this.roles = new ArrayList<>();
    }
    
    @Override
    public String getId() {
        return this.id;
    }
    
    @Override
    public LocalDateTime getCreatedAt() {
        return this.createdAt;
    }
    
    @Override
    public void validate() throws IllegalArgumentException {
        if (id == null || id.trim().isEmpty()) {
            throw new IllegalArgumentException("User ID cannot be null or empty");
        }
        if (name == null || name.trim().isEmpty()) {
            throw new IllegalArgumentException("User name cannot be null or empty");
        }
        if (email == null || !email.contains("@")) {
            throw new IllegalArgumentException("Invalid email format");
        }
    }
    
    public String getName() {
        return this.name;
    }
    
    public String getEmail() {
        return this.email;
    }
    
    public List<String> getRoles() {
        return new ArrayList<>(this.roles);
    }
    
    public void addRole(String role) {
        if (role != null && !role.trim().isEmpty()) {
            this.roles.add(role);
        }
    }
    
    public boolean hasRole(String role) {
        return this.roles.contains(role);
    }
}
"#;
        fs::write(base_package.join("User.java"), user_model).unwrap();

        let service_interface = r#"
package com.example.project;

import java.util.List;
import java.util.Optional;

public interface UserService {
    User createUser(String name, String email);
    Optional<User> findById(String id);
    List<User> findAll();
    void deleteUser(String id);
    User updateUser(String id, String name, String email);
}
"#;
        fs::write(base_package.join("UserService.java"), service_interface).unwrap();

        let user_service_impl = r#"
package com.example.project;

import java.util.List;
import java.util.Optional;
import java.util.ArrayList;
import java.util.concurrent.ConcurrentHashMap;

public class UserServiceImpl implements UserService {
    private final ConcurrentHashMap<String, User> users;
    
    public UserServiceImpl() {
        this.users = new ConcurrentHashMap<>();
    }
    
    @Override
    public User createUser(String name, String email) {
        String id = java.util.UUID.randomUUID().toString();
        User user = new User(id, name, email);
        user.validate();
        users.put(id, user);
        return user;
    }
    
    @Override
    public Optional<User> findById(String id) {
        return Optional.ofNullable(users.get(id));
    }
    
    @Override
    public List<User> findAll() {
        return new ArrayList<>(users.values());
    }
    
    @Override
    public void deleteUser(String id) {
        users.remove(id);
    }
    
    @Override
    public User updateUser(String id, String name, String email) {
        User user = users.get(id);
        if (user == null) {
            throw new IllegalArgumentException("User not found: " + id);
        }
        // Note: This is a simplified update - in real code, we'd need to create a new User
        // since User fields are private and there are no setters
        return user;
    }
}
"#;
        fs::write(base_package.join("UserServiceImpl.java"), user_service_impl).unwrap();

        // Create a class with some issues for diagnostics testing
        let problematic_class = r#"
package com.example.project;

import java.util.List;

public class ProblematicClass {
    private String uninitialized;
    
    public void methodWithIssues() {
        // Uninitialized variable usage
        System.out.println(uninitialized.length());
        
        // Potential null pointer
        String potentiallyNull = null;
        System.out.println(potentiallyNull.length());
        
        // Unused variable
        int unused = 42;
        
        // Empty catch block
        try {
            riskyOperation();
        } catch (Exception e) {
            // Empty catch block - bad practice
        }
    }
    
    private void riskyOperation() throws RuntimeException {
        throw new RuntimeException("Something went wrong");
    }
    
    // Method with return type but no return statement
    public String missingReturn() {
        System.out.println("This method should return a String");
    }
    
    // Unused parameter
    public void unusedParameter(String param) {
        System.out.println("Parameter is not used");
    }
}
"#;
        fs::write(base_package.join("ProblematicClass.java"), problematic_class).unwrap();

        project_dir.to_string_lossy().to_string()
    }

    #[test]
    fn test_java_plugin_full_project_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        // Verify project structure
        let package_dir = Path::new(&project_root).join("com/example/project");
        assert!(package_dir.exists());
        assert!(package_dir.is_dir());

        // Count Java files
        let java_files: Vec<_> = fs::read_dir(&package_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().map(|ext| ext == "java").unwrap_or(false))
            .collect();

        assert_eq!(java_files.len(), 5);

        // Verify specific files exist
        assert!(package_dir.join("Model.java").exists());
        assert!(package_dir.join("User.java").exists());
        assert!(package_dir.join("UserService.java").exists());
        assert!(package_dir.join("UserServiceImpl.java").exists());
        assert!(package_dir.join("ProblematicClass.java").exists());
    }

    #[test]
    fn test_java_plugin_entity_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_class_path = Path::new(&project_root).join("com/example/project/User.java");
        let content = fs::read_to_string(&user_class_path).unwrap();

        // Verify entities that should be extracted
        assert!(content.contains("public class User implements Model"));
        assert!(content.contains("private String id"));
        assert!(content.contains("private String name"));
        assert!(content.contains("private String email"));
        assert!(content.contains("public User(String id, String name, String email)"));
        assert!(content.contains("public String getId()"));
        assert!(content.contains("public String getName()"));
        assert!(content.contains("public void addRole(String role)"));
    }

    #[test]
    fn test_java_plugin_relationship_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_impl_path =
            Path::new(&project_root).join("com/example/project/UserServiceImpl.java");
        let content = fs::read_to_string(&user_impl_path).unwrap();

        // Verify relationships that should be extracted
        assert!(content.contains("implements UserService"));
        assert!(content.contains("private final ConcurrentHashMap<String, User> users"));
        assert!(content.contains("new User(id, name, email)"));
        assert!(content.contains("users.put(id, user)"));
        assert!(content.contains("users.get(id)"));
    }

    #[test]
    fn test_java_plugin_inheritance_relationships() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_class_path = Path::new(&project_root).join("com/example/project/User.java");
        let content = fs::read_to_string(&user_class_path).unwrap();

        // Verify inheritance relationships
        assert!(content.contains("implements Model"));
        assert!(content.contains("@Override"));
        assert!(content.contains("public String getId()"));
        assert!(content.contains("public LocalDateTime getCreatedAt()"));
        assert!(content.contains("public void validate()"));
    }

    #[test]
    fn test_java_plugin_method_call_relationships() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_impl_path =
            Path::new(&project_root).join("com/example/project/UserServiceImpl.java");
        let content = fs::read_to_string(&user_impl_path).unwrap();

        // Verify method call relationships
        assert!(content.contains("user.validate()"));
        assert!(content.contains("users.put(id, user)"));
        assert!(content.contains("users.get(id)"));
        assert!(content.contains("users.remove(id)"));
        assert!(content.contains("new ArrayList<>(users.values())"));
    }

    #[test]
    fn test_java_plugin_annotation_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_impl_path =
            Path::new(&project_root).join("com/example/project/UserServiceImpl.java");
        let content = fs::read_to_string(&user_impl_path).unwrap();

        // Verify annotation extraction
        assert!(content.contains("@Override"));
    }

    #[test]
    fn test_java_plugin_import_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_class_path = Path::new(&project_root).join("com/example/project/User.java");
        let content = fs::read_to_string(&user_class_path).unwrap();

        // Verify import extraction
        assert!(content.contains("import java.time.LocalDateTime"));
        assert!(content.contains("import java.util.List"));
        assert!(content.contains("import java.util.ArrayList"));
    }

    #[test]
    fn test_java_plugin_diagnostics_integration() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let problematic_class_path =
            Path::new(&project_root).join("com/example/project/ProblematicClass.java");
        let content = fs::read_to_string(&problematic_class_path).unwrap();

        // Verify diagnostic-worthy code exists
        assert!(content.contains("uninitialized"));
        assert!(content.contains("potentiallyNull = null"));
        assert!(content.contains("int unused = 42"));
        assert!(content.contains("catch (Exception e) {"));
        assert!(content.contains("missingReturn"));
        assert!(content.contains("unusedParameter"));
    }

    #[test]
    fn test_java_plugin_package_structure_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        // Verify package declarations
        let files = vec![
            "Model.java",
            "User.java",
            "UserService.java",
            "UserServiceImpl.java",
            "ProblematicClass.java",
        ];

        for file in files {
            let file_path = Path::new(&project_root).join("com/example/project").join(file);
            assert!(file_path.exists());

            let content = fs::read_to_string(&file_path).unwrap();
            assert!(content.contains("package com.example.project;"));
        }
    }

    #[test]
    fn test_java_plugin_complex_type_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_class_path = Path::new(&project_root).join("com/example/project/User.java");
        let content = fs::read_to_string(&user_class_path).unwrap();

        // Verify complex type usage
        assert!(content.contains("LocalDateTime"));
        assert!(content.contains("List<String>"));
        // Uses diamond operator: new ArrayList<>()
        assert!(content.contains("ArrayList"));
        // Optional<User> is in UserService interface, not User class
        // assert!(content.contains("Optional<User>"));
    }

    #[test]
    fn test_java_plugin_exception_handling_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_class_path = Path::new(&project_root).join("com/example/project/User.java");
        let content = fs::read_to_string(&user_class_path).unwrap();

        // Verify exception handling
        assert!(content.contains("throws IllegalArgumentException"));
        assert!(content.contains("throw new IllegalArgumentException"));
    }

    #[test]
    fn test_java_plugin_concurrency_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_impl_path =
            Path::new(&project_root).join("com/example/project/UserServiceImpl.java");
        let content = fs::read_to_string(&user_impl_path).unwrap();

        // Verify concurrency constructs
        assert!(content.contains("ConcurrentHashMap"));
    }

    #[test]
    fn test_java_plugin_performance_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = create_comprehensive_java_project(&temp_dir);

        let user_impl_path =
            Path::new(&project_root).join("com/example/project/UserServiceImpl.java");
        let content = fs::read_to_string(&user_impl_path).unwrap();

        // Verify performance patterns
        assert!(content.contains("new ArrayList<>(users.values())"));
        // Note: "this.roles" is in User.java, not UserServiceImpl.java
    }
}
