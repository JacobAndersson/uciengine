python meta\gen.py

call meta\b

git add . -A

git commit -m "%*"

pause

git push origin master

git push gitlab master
