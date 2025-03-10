for file in $(find . -type f -name "*.rs" | sort); do
  echo "===== $file ====="
  cat "$file"
  echo ""
done | pbcopy

