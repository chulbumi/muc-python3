# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [방파이름] 방파초기화')
            return
            for g in GUILD.attr:
                guild = GUILD.attr[g]
                #방파원들의 소속과 직위를 없애야함...
                path = guild['방파맵']
                room = getRoom(path)
                if room == None:
                    contimue
                room.attr.__delitem__('방파주인')
                room.save()
                for r in room['방파입구']:
                    if r.find(':') == -1:
                        path = room.zone + ':' + r
                    else:
                        path = r
                    enter = getRoom(path)
                    if enter == None:
                        continue
                    enter.attr.__delitem__('방파주인')
                    enter.save()
            GUILD.attr = {}
            GUILD.save()
        else:
            if line in GUILD.attr:
                guild = GUILD.attr[line]
                #방파원들의 소속과 직위를 없애야함...
                path = guild['방파맵']
                room = getRoom(path)
                if room == None:
                    contimue
                try:
                    room.attr.__delitem__('방파주인')
                    room.save()
                except:
                    pass

                for r in room['방파입구']:
                    if r.find(':') == -1:
                        path = room.zone + ':' + r
                    else:
                        path = r
                    enter = getRoom(path)
                    if enter == None:
                        continue
                    try:
                        enter.attr.__delitem__('방파주인')
                        enter.save()
                    except:
                        pass
                GUILD.attr.__delitem__(line) 
                GUILD.save()
            else:
                ob.sendLine('* 그런 방파가 없습니다.')
                return
        ob.sendLine('* 방파가 초기화되었습니다.')
