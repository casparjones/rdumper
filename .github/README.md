# GitHub Actions Workflows

This directory contains GitHub Actions workflows for the rDumper project.

## Workflows Overview

### 1. `docker-build.yml` - Main CI/CD Pipeline
**Triggers:** Push to main/develop branches, tags, pull requests

**Jobs:**
- **test**: Frontend and backend testing
  - Node.js setup and frontend build
  - Rust setup and cargo check/test
  - Dependency caching for performance
- **build**: Docker image building and pushing
  - Multi-platform builds (linux/amd64, linux/arm64)
  - Automatic tagging based on branch/tag/commit
  - Push to GitHub Container Registry
- **security-scan**: Container vulnerability scanning
  - Trivy security scan of built images
  - SARIF report upload to GitHub Security tab

### 2. `security.yml` - Comprehensive Security Scanning
**Triggers:** Daily schedule (2 AM UTC), push to main, pull requests

**Jobs:**
- **dependency-scan**: Dependency vulnerability scanning
  - npm audit for frontend dependencies
  - cargo audit for Rust dependencies
- **code-quality**: Code quality checks
  - ESLint for frontend code
  - Clippy and rustfmt for Rust code
- **container-scan**: Container and filesystem scanning
  - Trivy vulnerability scan on container images
  - Filesystem security scan
  - SARIF report uploads

### 3. `tests.yml` - Comprehensive Testing Suite
**Triggers:** Push to main/develop branches, pull requests

**Jobs:**
- **frontend-tests**: Frontend testing and building
  - npm test execution
  - Frontend build verification
- **backend-tests**: Backend testing and quality checks
  - Rust unit tests
  - Clippy linting
  - Code formatting checks
- **integration-tests**: Integration testing
  - MySQL service setup
  - End-to-end testing with real database
- **docker-build-test**: Docker image testing
  - Docker build verification
  - Container functionality testing

## Environment Variables

- `REGISTRY`: GitHub Container Registry (`ghcr.io`)
- `IMAGE_NAME`: Repository name for Docker images
- `GITHUB_TOKEN`: Automatically provided by GitHub Actions

## Security Features

- **Vulnerability Scanning**: Automated security scanning with Trivy
- **Dependency Auditing**: Regular checks for vulnerable dependencies
- **Code Quality**: Automated linting and formatting checks
- **Container Security**: Multi-layer security scanning
- **SARIF Reports**: Integration with GitHub Security tab

## Performance Optimizations

- **Caching**: Rust dependencies and npm packages are cached
- **Parallel Jobs**: Independent jobs run in parallel when possible
- **Multi-platform**: Docker images built for multiple architectures
- **Build Caching**: Docker layer caching for faster builds

## Troubleshooting

### Common Issues

1. **Trivy Image Not Found**: 
   - Ensure Docker build completes before security scan
   - Check image tags match between build and scan steps
   - Verify registry permissions

2. **Test Failures**:
   - Check service dependencies (MySQL for integration tests)
   - Verify environment setup and dependencies
   - Review test logs for specific error messages

3. **Build Failures**:
   - Check Rust/Node.js version compatibility
   - Verify all dependencies are properly declared
   - Review build logs for compilation errors

### Debugging Tips

- Check the Actions tab in GitHub for detailed logs
- Use `act` locally to test workflows before pushing
- Monitor the Security tab for vulnerability reports
- Review SARIF reports in the Security tab

## Contributing

When adding new workflows or modifying existing ones:

1. Test locally with `act` if possible
2. Ensure proper error handling and timeouts
3. Add appropriate permissions for security scans
4. Update this README with any new workflows or changes
5. Consider performance impact of new steps

## Security Considerations

- All workflows use least-privilege permissions
- Sensitive data is handled through GitHub Secrets
- Security scans are configured with appropriate severity levels
- SARIF reports are automatically uploaded to GitHub Security tab
